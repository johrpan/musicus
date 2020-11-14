use actix_web::{dev::HttpResponseBuilder, error, http::StatusCode, HttpResponse};
use derive_more::{Display, Error};

/// An error intended for the public interface.
#[derive(Display, Error, Debug)]
pub enum ServerError {
    NotFound,
    Unauthorized,
    Forbidden,
    Internal,
}

impl error::ResponseError for ServerError {
    fn error_response(&self) -> HttpResponse {
        HttpResponseBuilder::new(self.status_code()).finish()
    }

    fn status_code(&self) -> StatusCode {
        match self {
            ServerError::NotFound => StatusCode::NOT_FOUND,
            ServerError::Unauthorized => StatusCode::UNAUTHORIZED,
            ServerError::Forbidden => StatusCode::FORBIDDEN,
            ServerError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<r2d2::Error> for ServerError {
    fn from(error: r2d2::Error) -> Self {
        ServerError::Internal
    }
}

impl From<anyhow::Error> for ServerError {
    fn from(error: anyhow::Error) -> Self {
        ServerError::Internal
    }
}

impl From<error::BlockingError<ServerError>> for ServerError {
    fn from(error: error::BlockingError<ServerError>) -> Self {
        match error {
            error::BlockingError::Error(error) => error,
            error::BlockingError::Canceled => ServerError::Internal,
        }
    }
}
