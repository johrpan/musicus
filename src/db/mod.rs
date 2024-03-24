pub mod models;
pub mod schema;
pub mod tables;

use std::collections::HashMap;

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

use crate::util;

// This makes the SQL migration scripts accessible from the code.
const MIGRATIONS: EmbeddedMigrations = diesel_migrations::embed_migrations!();

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

/// A single translated string value.
#[derive(Serialize, Deserialize, AsExpression, FromSqlRow, Clone, Debug)]
#[diesel(sql_type = Text)]
pub struct TranslatedString(HashMap<String, String>);

impl TranslatedString {
    /// Get the best translation for the user's current locale.
    ///
    /// This will fall back to the generic variant if no translation exists. If no
    /// generic translation exists (which is a bug in the data), an empty string is
    /// returned and a warning is logged.
    pub fn get(&self) -> &str {
        match self.0.get(&*util::LANG) {
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
