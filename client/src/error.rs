use isahc::http::StatusCode;

/// An error within the client.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("The users login credentials were wrong.")]
    LoginFailed,

    #[error("The user has to be logged in to perform this action.")]
    Unauthorized,

    #[error("The user is not allowed to perform this action.")]
    Forbidden,

    #[error("The server returned an unexpected status code: {0}.")]
    UnexpectedResponse(StatusCode),

    #[error("A networking error happened.")]
    NetworkError(#[from] isahc::Error),

    #[error("A networking error happened.")]
    HttpError(#[from] isahc::http::Error),

    #[error("An error happened when serializing/deserializing.")]
    SerdeError(#[from] serde_json::Error),

    #[error("An IO error happened.")]
    IoError(#[from] std::io::Error),

    #[error("An error happened: {0}")]
    Other(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;
