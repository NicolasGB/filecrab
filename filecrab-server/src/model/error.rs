use s3::{creds::error::CredentialsError, error::S3Error};
use thiserror::Error;

pub type Result<T> = core::result::Result<T, ModelManagerError>;

#[derive(Error, Debug)]
pub enum ModelManagerError {
    //Minio Related errors
    #[error("invalid minio credentials: {0}")]
    MinioCredentials(#[from] CredentialsError),
    #[error(transparent)]
    S3Error(#[from] S3Error),

    //SurrealDB
    #[error("error connecting to new database")]
    NewDB(#[source] surrealdb::Error),
    #[error("error setting namespace: {ns} and database: {db}")]
    SetUseNSandDb {
        ns: String,
        db: String,
        source: surrealdb::Error,
    },

    #[cfg(not(feature = "rocksdb"))]
    #[error("error signing in the database")]
    SignIn(#[source] surrealdb::Error),

    #[error("error defining table")]
    CouldNotDefineTable(#[source] surrealdb::Error),
    #[error("error setting index")]
    CouldNotSetTableIndex(#[source] surrealdb::Error),
    #[error("error using take method on surrealdb result")]
    TakeError(#[source] surrealdb::Error),

    //Assets
    #[error("create asset error")]
    CreateAsset(#[source] surrealdb::Error),
    #[error("search asset error")]
    SearchAsset(#[source] surrealdb::Error),
    #[error("delete asset error")]
    DeleteAsset(#[source] surrealdb::Error),
    #[error("asset not found")]
    AssetNotFound,

    //Texts
    #[error("create text error")]
    CreateText(#[source] surrealdb::Error),
    #[error("search text error")]
    SearchText(#[source] surrealdb::Error),
    #[error("delete text error")]
    DeleteText(#[source] surrealdb::Error),
    #[error("text not found")]
    TextNotFound,

    //Stdio
    #[error("std io error")]
    StdIo(#[from] std::io::Error),
}
