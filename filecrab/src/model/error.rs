use s3::{creds::error::CredentialsError, error::S3Error};

pub type Result<T> = core::result::Result<T, ModelManagerError>;

#[derive(Debug)]
pub enum ModelManagerError {
    //Minio Related errors
    MinioCredentials(CredentialsError),
    CreateBucket(S3Error),
    NewBucket(S3Error),
    BucketExists(S3Error),

    //SurrealDB
    NewDB(surrealdb::Error),
    SetUseNSandDb(surrealdb::Error),
    SignIn(surrealdb::Error),
    CouldNotDefineTable(surrealdb::Error),
    CouldNotSetTableIndex(surrealdb::Error),
    TakeError(surrealdb::Error),

    //Assets
    CreateAsset(surrealdb::Error),
    SearchAsset(surrealdb::Error),
    AssetNotFound,

    //Password
    CouldNotHashPassword,
    InvalidPasswod,
}

impl core::fmt::Display for ModelManagerError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for ModelManagerError {}
