pub mod models;
pub mod schema;
pub mod tables;
pub mod views;

use std::{
    collections::HashMap,
    fmt::Display,
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

// This makes the SQL migration scripts accessible from the code.
const MIGRATIONS: EmbeddedMigrations = diesel_migrations::embed_migrations!("../migrations");

/// The user's preferred language code, used to pick the best translation out of a
/// [`TranslatedString`]. Set once at application startup via [`set_language`].
static LANG: OnceLock<String> = OnceLock::new();

/// Set the user's preferred language code. This should be called once, early during
/// application startup, before any [`TranslatedString::get`] calls happen.
pub fn set_language(lang: impl Into<String>) {
    if LANG.set(lang.into()).is_err() {
        log::warn!("set_language was called more than once; ignoring further calls");
    }
}

/// The schema version this build understands.
///
/// Stored in every library database's `meta` table. Any migration that
/// changes the schema must bump both this constant and the value written by the
/// migration, so that an older build can recognise a database it cannot read.
pub const SCHEMA_VERSION: i32 = 1;

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

/// The schema version recorded in an existing database, or `None` for a
/// database that predates `meta` (or is brand new).
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
/// Fails if the database was written by a newer version of Musicus, rather than
/// migrating or reading a schema this build does not understand.
pub fn connect(file_name: &str) -> Result<SqliteConnection, LibraryError> {
    log::info!("Opening database file '{}'", file_name);
    let mut connection = SqliteConnection::establish(file_name).map_err(anyhow::Error::from)?;

    if let Some(version) = schema_version(&mut connection)? {
        if version > SCHEMA_VERSION {
            return Err(LibraryError::SchemaTooNew {
                found: version,
                supported: SCHEMA_VERSION,
            });
        }
    }

    log::info!("Running migrations if necessary");
    connection
        .run_pending_migrations(MIGRATIONS)
        .map_err(|e| anyhow!(e))?;

    // Enable after running migrations to simplify changes in schema.
    diesel::sql_query("PRAGMA foreign_keys = ON").execute(&mut connection)?;

    Ok(connection)
}

/// The current time, in the form all database timestamps are stored in.
///
/// Timestamps are stored in UTC. They used to be naive local time, which made
/// `last_played_at` non-monotonic across DST transitions and machine
/// relocations, and was inconsistent with the `UNIXEPOCH()` calls that
/// `generate_recording` scores playlists with. Always go through this function
/// rather than reaching for `Local::now()` or `Utc::now()` directly.
pub fn now() -> chrono::NaiveDateTime {
    chrono::Utc::now().naive_utc()
}

/// Generate a random string suitable as an item ID.
pub fn generate_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

pub fn lock_connection(connection: &Mutex<SqliteConnection>) -> MutexGuard<'_, SqliteConnection> {
    connection
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A single translated string value.
#[derive(Serialize, Deserialize, AsExpression, FromSqlRow, Clone, Default, Debug)]
#[diesel(sql_type = Text)]
pub struct TranslatedString(pub HashMap<String, String>);

impl TranslatedString {
    /// Get the best translation for the user's current locale.
    ///
    /// This will fall back to the generic variant if no translation exists. If no
    /// generic translation exists (which is a bug in the data), an empty string is
    /// returned and a warning is logged.
    pub fn get(&self) -> &str {
        match LANG.get().and_then(|lang| self.0.get(lang)) {
            Some(s) => s,
            None => match self.0.get("generic") {
                Some(s) => s,
                None => {
                    log::warn!("No generic variant for TranslatedString: {:?}", self);
                    ""
                }
            },
        }
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
        let translated_string = serde_json::from_str(&text)?;
        Ok(translated_string)
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
