use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::sync::{Arc, Mutex};

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
const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

/// Generate a random string suitable as an item ID.
pub fn generate_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// Interface to a Musicus database.
pub struct Database {
    connection: Arc<Mutex<SqliteConnection>>,
}

impl Database {
    /// Create a new database interface and run migrations if necessary.
    pub fn new(file_name: &str) -> Result<Database> {
        info!("Opening database file '{}'", file_name);
        let mut connection = SqliteConnection::establish(file_name)?;
        diesel::sql_query("PRAGMA foreign_keys = ON").execute(&mut connection)?;

        info!("Running migrations if necessary");
        connection.run_pending_migrations(MIGRATIONS)?;

        Ok(Database {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    /// Defer all foreign keys for the next transaction.
    fn defer_foreign_keys(&self) -> Result<()> {
        diesel::sql_query("PRAGMA defer_foreign_keys = ON")
            .execute(&mut *self.connection.lock().unwrap())?;
        Ok(())
    }
}
