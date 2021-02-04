use isahc::http::StatusCode;

/// An error that can happen within the backend.
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

    #[error(transparent)]
    DatabaseError(#[from] crate::backend::DatabaseError),

    #[error("An IO error happened.")]
    IoError(#[from] std::io::Error),

    #[error("An error happened using the SecretService.")]
    SecretServiceError(#[from] secret_service::Error),

    #[error("An error happened while serializing or deserializing.")]
    SerdeError(#[from] serde_json::Error),

    #[error("An error happened in GLib.")]
    GlibError(#[from] glib::BoolError),

    #[error("A channel was canceled.")]
    ChannelError(#[from] futures_channel::oneshot::Canceled),

    #[error("Error decoding to UTF8.")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("An error happened: {0}")]
    Other(&'static str),

    // TODO: Remove this once anyhow has been dropped as a dependency.
    #[error("An unkown error happened.")]
    Unknown(#[from] anyhow::Error),
}


pub type Result<T> = std::result::Result<T, Error>;

