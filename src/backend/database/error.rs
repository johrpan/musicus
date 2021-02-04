use thiserror::Error;

/// Error that happens within the database module.
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error(transparent)]
    ConnectionError(#[from] diesel::result::ConnectionError),

    #[error(transparent)]
    MigrationsError(#[from] diesel_migrations::RunMigrationsError),

    #[error(transparent)]
    QueryError(#[from] diesel::result::Error),

    #[error(transparent)]
    SendError(#[from] std::sync::mpsc::SendError<super::thread::Action>),

    #[error(transparent)]
    ReceiveError(#[from] futures_channel::oneshot::Canceled),

    #[error("Database error: {0}")]
    Other(String),
}

/// Return type for database methods.
pub type DatabaseResult<T> = Result<T, DatabaseError>;
