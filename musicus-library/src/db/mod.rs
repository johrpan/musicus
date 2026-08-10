pub mod models;
pub mod schema;
pub mod tables;

#[cfg(test)]
mod migration_tests;

use std::{
    collections::HashMap,
    fmt::Display,
    sync::{Mutex, MutexGuard, OnceLock},
};

use anyhow::{anyhow, Result};
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

/// Connect to a Musicus database and apply any pending migrations.
pub fn connect(file_name: &str) -> Result<SqliteConnection> {
    log::info!("Opening database file '{}'", file_name);
    let mut connection = SqliteConnection::establish(file_name)?;

    log::info!("Running migrations if necessary");
    connection
        .run_pending_migrations(MIGRATIONS)
        .map_err(|e| anyhow!(e))?;

    // Enable after running migrations to simplify changes in schema.
    diesel::sql_query("PRAGMA foreign_keys = ON").execute(&mut connection)?;

    Ok(connection)
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

#[cfg(test)]
mod tests {
    use chrono::Local;

    use super::*;

    fn migrated_conn() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        conn.run_pending_migrations(MIGRATIONS).unwrap();
        conn
    }

    #[test]
    fn fresh_schema_applies_all_migrations() {
        let mut conn = migrated_conn();

        assert!(!conn.has_pending_migration(MIGRATIONS).unwrap());
        assert_eq!(conn.applied_migrations().unwrap().len(), 6);

        assert_eq!(
            schema::persons::table
                .count()
                .get_result::<i64>(&mut conn)
                .unwrap(),
            0
        );
    }

    #[test]
    fn translated_string_locale_fallback() {
        set_language("de");

        let mut translations = HashMap::new();
        translations.insert("de".to_string(), "Hallo".to_string());
        translations.insert("generic".to_string(), "Hello".to_string());
        assert_eq!(TranslatedString(translations).get(), "Hallo");

        let mut generic_only = HashMap::new();
        generic_only.insert("generic".to_string(), "Hello".to_string());
        assert_eq!(TranslatedString(generic_only).get(), "Hello");

        assert_eq!(TranslatedString(HashMap::new()).get(), "");
    }

    #[test]
    fn translated_string_round_trips_through_sqlite() {
        let mut conn = migrated_conn();
        let now = Local::now().naive_local();

        let mut name = HashMap::new();
        name.insert("generic".to_string(), "Ludwig van Beethoven".to_string());
        name.insert("de".to_string(), "Ludwig van Beethoven".to_string());
        let translated = TranslatedString(name);

        diesel::insert_into(schema::persons::table)
            .values((
                schema::persons::person_id.eq("test-person"),
                schema::persons::name.eq(&translated),
                schema::persons::source.eq("user"),
                schema::persons::enable_updates.eq(true),
                schema::persons::created_at.eq(now),
                schema::persons::edited_at.eq(now),
                schema::persons::last_used_at.eq(now),
            ))
            .execute(&mut conn)
            .unwrap();

        let loaded: TranslatedString = schema::persons::table
            .filter(schema::persons::person_id.eq("test-person"))
            .select(schema::persons::name)
            .first(&mut conn)
            .unwrap();

        assert_eq!(
            loaded.0.get("de"),
            Some(&"Ludwig van Beethoven".to_string())
        );
    }
}
