use anyhow::Result;
use diesel::r2d2;
use diesel::PgConnection;

pub mod ensembles;
pub use ensembles::*;

pub mod instruments;
pub use instruments::*;

pub mod persons;
pub use persons::*;

pub mod recordings;
pub use recordings::*;

pub mod users;
pub use users::*;

pub mod works;
pub use works::*;

mod schema;

// This makes the SQL migration scripts accessible from the code.
embed_migrations!();

/// A pool of connections to the database.
pub type DbPool = r2d2::Pool<r2d2::ConnectionManager<PgConnection>>;

/// One database connection from the connection pool.
pub type DbConn = r2d2::PooledConnection<r2d2::ConnectionManager<PgConnection>>;

/// Create a connection pool for a database. This will look for the database URL in the
/// "MUSICUS_DATABASE_URL" environment variable and fail, if that is not set.
pub fn connect() -> Result<DbPool> {
    let url = std::env::var("MUSICUS_DATABASE_URL")?;
    let manager = r2d2::ConnectionManager::<PgConnection>::new(url);
    let pool = r2d2::Pool::new(manager)?;

    // Run embedded migrations.
    let conn = pool.get()?;
    embedded_migrations::run(&conn)?;

    Ok(pool)
}
