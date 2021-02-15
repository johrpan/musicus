/// An error that can happened within the backend.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    ClientError(#[from] musicus_client::Error),

    #[error(transparent)]
    DatabaseError(#[from] musicus_database::Error),

    #[cfg(target_os = "linux")]
    #[error("An error happened using the SecretService.")]
    SecretServiceError(#[from] secret_service::Error),

    #[error("A channel was canceled.")]
    ChannelError(#[from] futures_channel::oneshot::Canceled),

    #[error("An error happened while decoding to UTF-8.")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("An error happened: {0}")]
    Other(&'static str),
}


pub type Result<T> = std::result::Result<T, Error>;

