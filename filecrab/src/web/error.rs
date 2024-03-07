use std::sync::Arc;

use axum::{extract::multipart::MultipartError, http::StatusCode, response::IntoResponse};
use thiserror::Error;
use tracing::error;

use crate::model::ModelManagerError;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("the filename is not set in the request")]
    MissingFileName,

    #[error(transparent)]
    ModelManager(ModelManagerError),

    #[error("error reading multipart file")]
    ReadingMultipartFile(#[from] MultipartError),

    #[error("the set expire time: {0}, is invalid")]
    InvalidExpireTime(String),

    #[error("todo remove me")]
    Anyhow(anyhow::Error),

    #[error(transparent)]
    Http(axum::http::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        error!("-->> {:12} - {self:?}", "INTO_RES");

        match self {
            Self::MissingFileName => {
                let mut response = (
                    StatusCode::BAD_REQUEST,
                    "File name was not set for the given object",
                )
                    .into_response();

                response.extensions_mut().insert(Arc::new(self));
                response
            }
            Self::ModelManager(ref mm_err) => {
                let code = match mm_err {
                    ModelManagerError::CreateAsset(_) => StatusCode::CONFLICT,
                    ModelManagerError::SearchAsset(_) => StatusCode::BAD_REQUEST,
                    ModelManagerError::DeleteAsset(_) => StatusCode::BAD_REQUEST,
                    ModelManagerError::AssetNotFound => StatusCode::NOT_FOUND,
                    ModelManagerError::CreateText(_) => StatusCode::CONFLICT,
                    ModelManagerError::SearchText(_) => StatusCode::BAD_REQUEST,
                    ModelManagerError::TextNotFound => StatusCode::NOT_FOUND,
                    ModelManagerError::InvalidPasswod => StatusCode::FORBIDDEN,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                };

                let mut response = (code, mm_err.to_string()).into_response();
                response.extensions_mut().insert(Arc::new(self));
                response
            }
            _ => {
                let mut response = StatusCode::INTERNAL_SERVER_ERROR.into_response();

                response.extensions_mut().insert(Arc::new(self));

                response
            }
        }
    }
}

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Error::Anyhow(value)
    }
    // add code here
}
