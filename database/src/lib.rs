// Required for schema.rs
#[macro_use]
extern crate diesel;

// Required for embed_migrations macro in database.rs
#[macro_use]
extern crate diesel_migrations;

use diesel::prelude::*;
use log::info;

pub mod ensembles;
pub use ensembles::*;

pub mod error;
pub use error::*;

pub mod instruments;
pub use instruments::*;

pub mod medium;
pub use medium::*;

pub mod persons;
pub use persons::*;

pub mod recordings;
pub use recordings::*;

pub mod works;
pub use works::*;

mod schema;

// This makes the SQL migration scripts accessible from the code.
embed_migrations!();

/// Generate a random string suitable as an item ID.
pub fn generate_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// Interface to a Musicus database.
pub struct Database {
    connection: SqliteConnection,
}

impl Database {
    /// Create a new database interface and run migrations if necessary.
    pub fn new(file_name: &str) -> Result<Database> {
        info!("Opening database file '{}'", file_name);
        let connection = SqliteConnection::establish(file_name)?;
        diesel::sql_query("PRAGMA foreign_keys = ON").execute(&connection)?;

        info!("Running migrations if necessary");
        embedded_migrations::run(&connection)?;

        Ok(Database { connection })
    }

    /// Defer all foreign keys for the next transaction.
    fn defer_foreign_keys(&self) -> Result<()> {
        diesel::sql_query("PRAGMA defer_foreign_keys = ON").execute(&self.connection)?;
        Ok(())
    }
}
