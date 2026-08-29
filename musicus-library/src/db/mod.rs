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
use serde::{Deserialize, Serialize};

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
/// variants are optional. Backed by a `BTreeMap` rather than a `HashMap` so
/// that serialization is deterministic — this lets
/// `work_tags.value`/`recording_tags.value` be compared for exact equality
/// directly in SQL.
#[derive(Serialize, Deserialize, AsExpression, FromSqlRow, Clone, Debug, PartialEq, Eq, Hash)]
#[diesel(sql_type = Text)]
pub struct TranslatedString(BTreeMap<String, String>);

impl TranslatedString {
    /// Build a `TranslatedString` carrying only the generic variant.
    ///
    /// The generic value must be nonblank; callers that cannot guarantee that
    /// should use [`TranslatedString::try_from_map`] instead.
    pub fn generic(text: impl Into<String>) -> Self {
        let text = text.into();

        assert!(
            !text.trim().is_empty(),
            "TranslatedString needs a nonblank generic value"
        );

        Self(BTreeMap::from([(GENERIC.to_string(), text)]))
    }

    /// Build a `TranslatedString` from a map of locale values, requiring a
    /// nonblank generic value. Locale keys are normalized.
    pub fn try_from_map(map: BTreeMap<String, String>) -> Result<Self, LibraryError> {
        let map = map
            .into_iter()
            .map(|(key, value)| (normalize_tag(&key), value))
            .collect::<BTreeMap<String, String>>();

        match map.get(GENERIC) {
            Some(generic) if !generic.trim().is_empty() => Ok(Self(map)),
            _ => Err(LibraryError::InvalidInput(
                "TranslatedString needs a nonblank generic value".to_owned(),
            )),
        }
    }

    /// Read-only iteration over every stored variant, including `generic`, in
    /// deterministic (sorted key) order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }

    /// The generic (language-independent) value.
    pub fn generic_value(&self) -> Option<&str> {
        self.0.get(GENERIC).map(String::as_str)
    }

    /// Set a locale variant, adding or replacing it. The generic value cannot
    /// be removed through this method; setting the generic value is a no-op.
    pub fn set_locale(&mut self, lang: &str, text: impl Into<String>) {
        let key = normalize_tag(lang);

        if key == GENERIC {
            return;
        }

        self.0.insert(key, text.into());
    }

    /// Remove a locale variant. The generic value is never removed.
    pub fn remove_locale(&mut self, lang: &str) {
        let key = normalize_tag(lang);

        if key != GENERIC {
            self.0.remove(&key);
        }
    }

    /// Get the best translation for the user's current locale.
    ///
    /// A regional tag falls back to its base language before the generic
    /// variant (`de-DE -> de -> generic`). The generic variant is guaranteed
    /// to exist, so this never falls back to an empty string.
    pub fn get(&self) -> &str {
        let Some(lang) = LANG.get() else {
            return self.generic_value().unwrap_or_default();
        };

        if let Some(text) = self.0.get(lang) {
            return text;
        }

        if let Some(base) = lang.split('-').next().filter(|base| *base != lang) {
            if let Some(text) = self.0.get(base) {
                return text;
            }
        }

        self.generic_value().unwrap_or_default()
    }
}

impl Display for TranslatedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get())
    }
}

impl<DB: Backend> FromSql<Text, DB> for TranslatedString
where
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let text = String::from_sql(bytes)?;

        // Every stored value written since translations became JSON is a JSON
        // object from locale key to text. Anything else is a legacy plain-text
        // value, which is migrated to a generic-only value in memory.
        match serde_json::from_str::<BTreeMap<String, String>>(&text) {
            Ok(map) => TranslatedString::try_from_map(map).map_err(|err| anyhow!(err).into()),
            Err(_) if text.trim().is_empty() => Err(anyhow!(
                "Translated string value is empty; the database is corrupt or too old"
            )
            .into()),
            Err(_) => Ok(TranslatedString::generic(text)),
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
