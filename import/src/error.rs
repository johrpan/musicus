use std::error;

/// An error within an import session.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// A timeout was reached.
    #[error("{0}")]
    Timeout(String),

    /// Some common error.
    #[error("{msg}")]
    Other {
        /// The error message.
        msg: String,

        #[source]
        source: Option<Box<dyn error::Error + Send + Sync>>,
    },

    /// Something unexpected happened.
    #[error("{msg}")]
    Unexpected {
        /// The error message.
        msg: String,

        #[source]
        source: Option<Box<dyn error::Error + Send + Sync>>,
    },
}

impl Error {
    /// Create a new error with an explicit source.
    pub(super) fn os(source: impl error::Error + Send + Sync + 'static) -> Self {
        Self::Unexpected {
            msg: format!("An error has happened: {}", source),
            source: Some(Box::new(source)),
        }
    }

    /// Create a new unexpected error without an explicit source.
    pub(super) fn u(msg: String) -> Self {
        Self::Unexpected { msg, source: None }
    }

    /// Create a new unexpected error with an explicit source.
    pub(super) fn us(source: impl error::Error + Send + Sync + 'static) -> Self {
        Self::Unexpected {
            msg: format!("An unexpected error has happened: {}", source),
            source: Some(Box::new(source)),
        }
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for Error {
    fn from(err: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::us(err)
    }
}

impl From<gstreamer::glib::Error> for Error {
    fn from(err: gstreamer::glib::Error) -> Self {
        Self::us(err)
    }
}

impl From<gstreamer::glib::BoolError> for Error {
    fn from(err: gstreamer::glib::BoolError) -> Self {
        Self::us(err)
    }
}

impl From<gstreamer::StateChangeError> for Error {
    fn from(err: gstreamer::StateChangeError) -> Self {
        Self::us(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::us(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
