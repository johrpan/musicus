pub mod schema;
pub mod tables;
pub mod views;

use std::{
    collections::BTreeMap,
    fmt::Display,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, OnceLock},
};

use anyhow::{anyhow, Result};

use crate::error::LibraryError;
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql, FromSqlRow},
    expression::AsExpression,
    prelude::*,
    serialize::{self, IsNull, Output, ToSql},
    sql_types::Text,
    sqlite::Sqlite,
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness};

use self::schema::tracks;
use crate::db::tables::{LibraryTrackPath, TRACKS_SUBDIR};
use std::collections::HashSet;

// This makes the SQL migration scripts accessible from the code.
const MIGRATIONS: EmbeddedMigrations = diesel_migrations::embed_migrations!("../migrations");

/// The user's preferred language code, used to pick the best translation out of a
/// [`TranslatedString`]. Set once at application startup via [`set_language`].
/// The tag is normalized (lowercase, underscores as hyphens).
static LANG: OnceLock<String> = OnceLock::new();

/// The key of the value every [`TranslatedString`] must carry: the
/// language-independent default text.
const GENERIC: &str = "generic";

/// Normalize a language tag so that equivalent spellings (".de", "DE",
/// "de_DE") point at the same map key.
fn normalize_tag(tag: &str) -> String {
    tag.trim().to_ascii_lowercase().replace('_', "-")
}

/// Set the user's preferred language code. This should be called once, early during
/// application startup, before any [`TranslatedString::get`] calls happen.
pub fn set_language(lang: impl Into<String>) {
    let lang = normalize_tag(&lang.into());

    if LANG.set(lang).is_err() {
        log::warn!("set_language was called more than once; ignoring further calls");
    }
}

/// The schema version this build understands.
///
/// Stored in every library database's `meta` table. Any migration that
/// changes the schema must bump both this constant and the value written by the
/// migration, so that an older build can recognise a database it cannot read.
pub const SCHEMA_VERSION: i32 = 3;

#[derive(QueryableByName)]
struct SchemaVersionRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    schema_version: i32,
}

#[derive(QueryableByName)]
struct CountRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    count: i32,
}

/// The schema version recorded in an existing database, or `None` if the
/// database is not populated yet.
fn schema_version(connection: &mut SqliteConnection) -> Result<Option<i32>> {
    let exists = diesel::sql_query(
        "SELECT COUNT(*) AS count FROM sqlite_master \
         WHERE type = 'table' AND name = 'meta'",
    )
    .get_result::<CountRow>(connection)?
    .count;

    if exists == 0 {
        return Ok(None);
    }

    Ok(
        diesel::sql_query("SELECT schema_version FROM meta WHERE id = 1")
            .get_result::<SchemaVersionRow>(connection)
            .optional()?
            .map(|row| row.schema_version),
    )
}

/// Connect to a Musicus database and apply any pending migrations.
///
/// Fails if the database was written by a newer version of Musicus, or if the
/// path is not representable as UTF-8 (which SQLite requires).
pub fn connect(path: impl AsRef<Path>) -> Result<SqliteConnection, LibraryError> {
    let file_name = path.as_ref().to_str().ok_or_else(|| {
        LibraryError::InvalidInput(format!(
            "Database path {} is not valid UTF-8",
            path.as_ref().display()
        ))
    })?;

    log::info!("Opening database file '{}'", file_name);
    let mut connection = SqliteConnection::establish(file_name).map_err(anyhow::Error::from)?;

    let previous_version = schema_version(&mut connection)?;

    if let Some(version) = previous_version {
        if version > SCHEMA_VERSION {
            return Err(LibraryError::SchemaTooNew {
                found: version,
                supported: SCHEMA_VERSION,
            });
        }
    }

    // Databases older than schema 3 store track paths in the legacy flat or
    // JSON-array form. Validate and rewrite them, and move their files into
    // the managed `tracks/` subtree, before the migration rebuilds `tracks`
    // with its path constraints. Fresh databases skip this: the tables do not
    // exist yet and start at the current schema.
    if previous_version.is_some_and(|version| version < 3) {
        let root = path.as_ref().parent().unwrap_or_else(|| Path::new("."));
        upgrade_track_paths(&mut connection, root)?;
    }

    log::info!("Running migrations if necessary");
    connection
        .run_pending_migrations(MIGRATIONS)
        .map_err(|e| anyhow!(e))?;

    // Enable after running migrations to simplify changes in schema.
    diesel::sql_query("PRAGMA foreign_keys = ON").execute(&mut connection)?;

    Ok(connection)
}

/// One legacy track path, validated and ready to move.
struct TrackPathUpgrade {
    track_id: String,
    /// The canonical path the row will store.
    new_path: LibraryTrackPath,
    /// The file's current location, if one is expected at all.
    legacy_file: PathBuf,
}

/// Validate every legacy track path, move the files into the managed
/// `tracks/` subtree, and rewrite the stored values so that the schema
/// migration can build the constrained `tracks` table.
///
/// Everything is validated up front (including duplicate detection after
/// conversion) before any file is moved or row rewritten. Moves and row
/// updates happen in one transaction; if a move fails, the files already
/// moved are renamed back, so retrying starts from a consistent state.
fn upgrade_track_paths(connection: &mut SqliteConnection, root: &Path) -> Result<(), LibraryError> {
    let rows = tracks::table
        .select((tracks::track_id, tracks::path))
        .load::<(String, String)>(connection)
        .map_err(anyhow::Error::from)?;

    let mut upgrades = Vec::with_capacity(rows.len());
    let mut seen_paths = HashSet::new();

    for (track_id, stored) in rows {
        let upgrade = upgrade_track_path(&track_id, &stored)?;

        if !seen_paths.insert(upgrade.new_path.as_str().to_owned()) {
            return Err(LibraryError::InvalidInput(format!(
                "Two tracks share the file '{}' after conversion; the library \
                 cannot be migrated automatically",
                upgrade.new_path
            )));
        }

        upgrades.push(upgrade);
    }

    let mut moved = Vec::<(PathBuf, PathBuf)>::new();

    let result = connection.transaction::<(), anyhow::Error, _>(|connection| {
        for upgrade in &upgrades {
            let new_file = upgrade.new_path.resolve(root);
            let legacy_file = root.join(&upgrade.legacy_file);

            if new_file.exists() || !legacy_file.exists() {
                continue;
            }

            if let Some(parent) = new_file.parent() {
                if parent != root {
                    std::fs::create_dir_all(parent)?;
                }
            }

            std::fs::rename(&legacy_file, &new_file)?;
            moved.push((legacy_file, new_file));

            diesel::update(tracks::table.filter(tracks::track_id.eq(&upgrade.track_id)))
                .set(tracks::path.eq(upgrade.new_path.as_str()))
                .execute(connection)?;
        }

        Ok(())
    });

    if let Err(err) = result {
        // The database rows are rolled back with the transaction, so the files
        // have to follow them back to the legacy location.
        for (from, to) in moved.into_iter().rev() {
            if let Err(rollback) = std::fs::rename(&to, &from) {
                log::error!(
                    "Failed to move track file back to {}: {rollback}",
                    from.display()
                );
            }
        }

        return Err(LibraryError::Other(err.into()));
    }

    Ok(())
}

/// Validate one stored track path and decide its canonical form and the
/// legacy file location to move from.
fn upgrade_track_path(track_id: &str, stored: &str) -> Result<TrackPathUpgrade, LibraryError> {
    // Legacy rows serialize a JSON array of host path components; a flat file
    // name is the only form this library ever wrote, so anything longer is
    // ambiguous.
    let text = stored.trim();

    if text.starts_with('[') {
        let components = serde_json::from_str::<Vec<String>>(text).map_err(|err| {
            LibraryError::InvalidInput(format!("Track {track_id} has a malformed JSON path: {err}"))
        })?;

        match components.as_slice() {
            [name] => {
                let name = name.trim();
                let legacy_file = PathBuf::from(name);
                let new_path =
                    LibraryTrackPath::managed(&[TRACKS_SUBDIR, name]).map_err(|err| {
                        LibraryError::InvalidInput(format!("Track {track_id}: {err}"))
                    })?;

                Ok(TrackPathUpgrade {
                    track_id: track_id.to_owned(),
                    new_path,
                    legacy_file,
                })
            }
            [] => Err(LibraryError::InvalidInput(format!(
                "Track {track_id} has an empty JSON path"
            ))),
            _ => Err(LibraryError::InvalidInput(format!(
                "Track {track_id} has a multi-component JSON path that cannot \
                 be migrated automatically"
            ))),
        }
    } else {
        let parsed = LibraryTrackPath::try_from_str(text)
            .map_err(|err| LibraryError::InvalidInput(format!("Track {track_id}: {err}")))?;

        let managed_rest = parsed.as_str().strip_prefix("tracks/").map(str::to_owned);

        match managed_rest.as_deref() {
            // Already in the managed subtree with a single component: the file
            // may still sit at its old flat place next to the database.
            Some(rest) if !rest.contains('/') => Ok(TrackPathUpgrade {
                track_id: track_id.to_owned(),
                new_path: parsed,
                legacy_file: PathBuf::from(rest),
            }),
            // A deeper managed path was never flat; nothing to move.
            Some(_) => Ok(TrackPathUpgrade {
                track_id: track_id.to_owned(),
                new_path: parsed,
                legacy_file: root_of_nothing(),
            }),
            // A legacy flat name next to the database.
            None => {
                let new_path =
                    LibraryTrackPath::managed(&[TRACKS_SUBDIR, text]).map_err(|err| {
                        LibraryError::InvalidInput(format!("Track {track_id}: {err}"))
                    })?;

                Ok(TrackPathUpgrade {
                    track_id: track_id.to_owned(),
                    new_path,
                    legacy_file: PathBuf::from(parsed.as_str()),
                })
            }
        }
    }
}

/// A legacy file that is never expected to exist.
fn root_of_nothing() -> PathBuf {
    PathBuf::from(".nonexistent-track-file")
}

/// The current time as used within the database.
pub fn now() -> chrono::NaiveDateTime {
    chrono::Utc::now().naive_utc()
}

/// Generate a random string suitable as an item ID.
pub fn generate_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// Convert a collection index into a database sequence number, rejecting
/// collections too large to store their order.
pub fn sequence_number(index: usize) -> Result<i32, LibraryError> {
    i32::try_from(index).map_err(|_| {
        LibraryError::InvalidInput(format!(
            "Too many ordered items ({index}) to store their sequence"
        ))
    })
}

pub fn lock_connection(connection: &Mutex<SqliteConnection>) -> MutexGuard<'_, SqliteConnection> {
    connection
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A single translated string value.
///
/// A [`TranslatedString`] always carries a nonblank `generic` value; locale
/// variants are optional. The generic value is stored separately so that it
/// can be read without a lookup: [`Self::generic_value`] cannot fail, and no
/// construction or mutation path can leave the generic value blank or missing.
///
/// Serialization is a flat map from locale key to text, deterministic and
/// key-sorted — this lets `work_tags.value`/`recording_tags.value` be
/// compared for exact equality directly in SQL.
#[derive(AsExpression, FromSqlRow, Clone, Debug, PartialEq, Eq, Hash)]
#[diesel(sql_type = Text)]
pub struct TranslatedString {
    /// The guaranteed nonblank language-independent text.
    generic: String,
    /// Locale variants, keyed by normalized language tag.
    locales: BTreeMap<String, String>,
}

impl TranslatedString {
    /// Build a `TranslatedString` carrying only the generic variant.
    ///
    /// Fails with [`LibraryError::InvalidInput`] when `text` is blank;
    /// callers that build the value from untrusted input can use
    /// [`TranslatedString::try_from_map`] instead.
    pub fn generic(text: impl Into<String>) -> crate::Result<Self> {
        let text = text.into();

        if text.trim().is_empty() {
            return Err(LibraryError::InvalidInput(
                "TranslatedString needs a nonblank generic value".to_owned(),
            ));
        }

        Ok(Self {
            generic: text,
            locales: BTreeMap::new(),
        })
    }

    /// Build a `TranslatedString` from a map of locale values, requiring a
    /// nonblank generic value.
    ///
    /// Keys are normalized (lowercase, underscores as hyphens) before
    /// validation; two entries that normalize to the same tag are rejected
    /// instead of silently overwriting one value with another. Blank values
    /// are rejected so that stored variants stay meaningful.
    pub fn try_from_map(map: BTreeMap<String, String>) -> crate::Result<Self> {
        let mut normalized = BTreeMap::new();
        for (key, value) in map {
            let key = normalize_tag(&key);
            if let Some(previous) = normalized.insert(key.clone(), value.clone()) {
                return Err(LibraryError::InvalidInput(format!(
                    "TranslatedString has two entries that normalize to the \
                     same tag {key:?} ({previous:?} and {value:?})"
                )));
            }
        }

        Self::from_normalized_map(normalized)
    }

    /// Validate a map of already-normalized entries and build the value.
    ///
    /// Exactly one usable (nonblank) `generic` entry is required, and every
    /// stored variant must be nonblank so that stored translations stay
    /// meaningful.
    fn from_normalized_map(map: BTreeMap<String, String>) -> crate::Result<Self> {
        let generic = match map.get(GENERIC) {
            Some(generic) if !generic.trim().is_empty() => generic.clone(),
            _ => {
                return Err(LibraryError::InvalidInput(
                    "TranslatedString needs a nonblank generic value".to_owned(),
                ))
            }
        };

        for (lang, text) in &map {
            if lang.is_empty() {
                return Err(LibraryError::InvalidInput(
                    "TranslatedString has an entry with an empty language tag".to_owned(),
                ));
            }
            if text.trim().is_empty() {
                return Err(LibraryError::InvalidInput(format!(
                    "TranslatedString has a blank value for {lang:?}"
                )));
            }
        }

        let mut locales = map;
        locales.remove(GENERIC);

        Ok(Self { generic, locales })
    }

    /// Read-only iteration over every stored variant, starting with
    /// `generic`, then each locale in deterministic (sorted key) order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        std::iter::once((GENERIC, self.generic.as_str())).chain(
            self.locales
                .iter()
                .map(|(lang, text)| (lang.as_str(), text.as_str())),
        )
    }

    /// The generic (language-independent) value.
    ///
    /// Always present: every construction and mutation path validates the
    /// storage invariant.
    pub fn generic_value(&self) -> &str {
        &self.generic
    }

    /// Set a locale variant, adding or replacing it.
    ///
    /// The generic value cannot be set through this method, and blank texts
    /// are rejected with [`LibraryError::InvalidInput`] so that a stored
    /// variant stays meaningful.
    pub fn set_locale(&mut self, lang: &str, text: impl Into<String>) -> crate::Result<()> {
        let key = normalize_tag(lang);

        if key == GENERIC {
            return Err(LibraryError::InvalidInput(
                "The generic translation cannot be set as a locale".to_owned(),
            ));
        }
        if key.is_empty() {
            return Err(LibraryError::InvalidInput(
                "The language tag must not be blank".to_owned(),
            ));
        }

        let text = text.into();
        if text.trim().is_empty() {
            return Err(LibraryError::InvalidInput(format!(
                "The translation for {key:?} must not be blank"
            )));
        }

        self.locales.insert(key, text);
        Ok(())
    }

    /// Remove a locale variant. The generic value is never removed.
    pub fn remove_locale(&mut self, lang: &str) {
        let key = normalize_tag(lang);

        if key != GENERIC {
            self.locales.remove(&key);
        }
    }

    /// Get the best translation for the user's current locale.
    ///
    /// A regional tag falls back to its base language before the generic
    /// variant (`de-DE -> de -> generic`). The generic variant is guaranteed
    /// to exist, so this never falls back to an empty string.
    pub fn get(&self) -> &str {
        let Some(lang) = LANG.get() else {
            return &self.generic;
        };

        if let Some(text) = self.locales.get(lang) {
            return text;
        }

        if let Some(base) = lang.split('-').next().filter(|base| *base != lang) {
            if let Some(text) = self.locales.get(base) {
                return text;
            }
        }

        &self.generic
    }
}

impl Display for TranslatedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get())
    }
}

impl serde::Serialize for TranslatedString {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(1 + self.locales.len()))?;
        map.serialize_entry(GENERIC, &self.generic)?;
        for (lang, text) in &self.locales {
            map.serialize_entry(lang, text)?;
        }
        map.end()
    }
}

impl<'de> serde::Deserialize<'de> for TranslatedString {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let map = BTreeMap::<String, String>::deserialize(deserializer)?;
        TranslatedString::try_from_map(map).map_err(serde::de::Error::custom)
    }
}

impl<DB: Backend> FromSql<Text, DB> for TranslatedString
where
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let text = String::from_sql(bytes)?;

        // Every stored value written since translations became JSON is a JSON
        // object from locale key to text. Anything that is valid JSON but not
        // that object shape is corrupt. Anything that is not valid JSON at
        // all is a legacy plain-text value, migrated to a validated
        // generic-only value in memory.
        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(serde_json::Value::Object(map)) => {
                let map = map
                    .into_iter()
                    .map(|(key, value)| match value {
                        serde_json::Value::String(text) => Ok((key, text)),
                        _ => Err(anyhow!(
                            "Translated string value {key:?} is not text; the \
                             database is corrupt"
                        )
                        .into()),
                    })
                    .collect::<deserialize::Result<BTreeMap<String, String>>>()?;

                TranslatedString::try_from_map(map)
                    .map_err(|err| anyhow!("Invalid translated string value: {err}").into())
            }
            Ok(_) => Err(anyhow!(
                "Translated string value is not a translation object; the \
                 database is corrupt"
            )
            .into()),
            Err(_) if text.trim().is_empty() => Err(anyhow!(
                "Translated string value is empty; the database is corrupt or too old"
            )
            .into()),
            Err(_) => TranslatedString::generic(text).map_err(|err| anyhow!("{err}").into()),
        }
    }
}

impl ToSql<Text, Sqlite> for TranslatedString
where
    String: ToSql<Text, Sqlite>,
{
    fn to_sql(&self, out: &mut Output<Sqlite>) -> serialize::Result {
        let text = serde_json::to_string(self)?;
        out.set_value(text);
        Ok(IsNull::No)
    }
}
