/// Error that happens within the database module.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    ConnectionError(#[from] diesel::result::ConnectionError),

    #[error(transparent)]
    MigrationsError(#[from] diesel_migrations::RunMigrationsError),

    #[error(transparent)]
    QueryError(#[from] diesel::result::Error),

    #[error("Missing item dependency ({0} {1})")]
    MissingItem(&'static str, String),

    #[error("Failed to parse {0} from '{1}'")]
    ParsingError(&'static str, String),

    #[error(transparent)]
    SendError(#[from] std::sync::mpsc::SendError<super::thread::Action>),

    #[error(transparent)]
    ReceiveError(#[from] tokio::sync::oneshot::error::RecvError),

    #[error("{0}")]
    Other(&'static str),
}

/// Return type for database methods.
pub type Result<T> = std::result::Result<T, Error>;
