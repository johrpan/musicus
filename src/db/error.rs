/// Error that happens within the database module.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Connection(#[from] diesel::result::ConnectionError),

    #[error(transparent)]
    Migrations(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error(transparent)]
    Query(#[from] diesel::result::Error),

    #[error("Missing item dependency ({0} {1})")]
    MissingItem(&'static str, String),

    #[error("Failed to parse {0} from '{1}'")]
    Parsing(&'static str, String),

    #[error("{0}")]
    Other(&'static str),
}

/// Return type for database methods.
pub type Result<T> = std::result::Result<T, Error>;
