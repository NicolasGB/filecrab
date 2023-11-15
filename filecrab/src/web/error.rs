use axum::{extract::multipart::MultipartError, http::StatusCode, response::IntoResponse};
use tracing::error;

use crate::model::ModelManagerError;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    FilenameNotFound,

    CouldNotGenerateToken,
    NonUtf8Token,

    ModelManager(ModelManagerError),
    ErrorReadingMultipartFile(MultipartError),

    ErrorAnyhow(anyhow::Error),

    Http(axum::http::Error),
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        error!("-->> {:12} - {self:?}", "INTO_RES");

        let mut response = StatusCode::INTERNAL_SERVER_ERROR.into_response();

        response.extensions_mut().insert(self);

        response
    }
}

// Convert multipart error
impl From<MultipartError> for Error {
    fn from(value: MultipartError) -> Self {
        Error::ErrorReadingMultipartFile(value)
    }
}

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Error::ErrorAnyhow(value)
    }
    // add code here
}
