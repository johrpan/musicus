use anyhow::Result;
use diesel::prelude::*;

pub mod ensembles;
pub use ensembles::*;

pub mod instruments;
pub use instruments::*;

pub mod persons;
pub use persons::*;

pub mod recordings;
pub use recordings::*;

pub mod thread;
pub use thread::*;

pub mod tracks;
pub use tracks::*;

pub mod works;
pub use works::*;

mod schema;

// This makes the SQL migration scripts accessible from the code.
embed_migrations!();

/// Generate a random string suitable as an item ID.
pub fn generate_id() -> String {
    let mut buffer = uuid::Uuid::encode_buffer();
    let id = uuid::Uuid::new_v4().to_simple().encode_lower(&mut buffer);

    id.to_string()
}

/// Interface to a Musicus database.
pub struct Database {
    connection: SqliteConnection,
}

impl Database {
    /// Create a new database interface and run migrations if necessary.
    pub fn new(file_name: &str) -> Result<Database> {
        let connection = SqliteConnection::establish(file_name)?;

        diesel::sql_query("PRAGMA foreign_keys = ON").execute(&connection)?;
        embedded_migrations::run(&connection)?;

        Ok(Database { connection })
    }

    /// Defer all foreign keys for the next transaction.
    fn defer_foreign_keys(&self) -> Result<()> {
        diesel::sql_query("PRAGMA defer_foreign_keys = ON").execute(&self.connection)?;
        Ok(())
    }
}
