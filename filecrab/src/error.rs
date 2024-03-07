use serde::Serialize;
use thiserror::Error;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Error, Debug, Serialize, Clone)]
pub enum Error {
    #[error("missing environment variable {0}")]
    ConfigMissingEnv(&'static str),

    #[error("the value of the environment {0} inavild")]
    InvalidEnvType(&'static str),

    #[error("error initializing the model manager")]
    CouldNotInitModelManager,

    #[error("error initializing server tcp listener: {0}")]
    CouldNotInitTcpListener(&'static str),
}
