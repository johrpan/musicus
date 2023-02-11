use diesel::prelude::*;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use log::info;

pub use diesel::SqliteConnection;

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

/// Connect to a Musicus database running migrations if necessary.
pub fn connect(file_name: &str) -> Result<SqliteConnection> {
    info!("Opening database file '{}'", file_name);
    let mut connection = SqliteConnection::establish(file_name)?;
    diesel::sql_query("PRAGMA foreign_keys = ON").execute(&mut connection)?;

    info!("Running migrations if necessary");
    connection.run_pending_migrations(MIGRATIONS)?;

    Ok(connection)
}

/// Generate a random string suitable as an item ID.
pub fn generate_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// Defer all foreign keys for the next transaction.
fn defer_foreign_keys(connection: &mut SqliteConnection) -> Result<()> {
    diesel::sql_query("PRAGMA defer_foreign_keys = ON").execute(connection)?;
    Ok(())
}
