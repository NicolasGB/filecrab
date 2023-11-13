pub mod asset;
mod error;

pub use error::{ModelManagerError, Result};

use axum::body::Bytes;
use surrealdb::{
    engine::remote::ws::{Client, Ws},
    opt::auth::Root,
    Surreal,
};

use crate::config::{self, config};
use s3::{creds::Credentials, Bucket, BucketConfiguration, Region};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct ModelManager {
    //Bucket is clonable as its references are behind an arc
    bucket: Bucket,
    //Surrealdb is also clonable
    db: Surreal<Client>,
}

impl ModelManager {
    pub async fn new() -> Result<Self> {
        let bucket = ModelManager::connect_minio().await?;

        let db = ModelManager::connect_db().await?;

        Ok(ModelManager { bucket, db })
    }

    /// Function that tries to connect to the SurrealDB instance and panics if it doesn't achieve
    /// it
    async fn connect_db() -> Result<Surreal<Client>> {
        //SurrealDB
        let db = Surreal::new::<Ws>(&config().DB_HOST)
            .await
            .map_err(ModelManagerError::NewDB)?;

        let _ = db
            .signin(Root {
                username: &config().DB_USER,
                password: &config().DB_PASSWORD,
            })
            .await
            .map_err(ModelManagerError::SignIn)?;

        //Set DB from config
        db.use_ns(&config().DB_NS)
            .use_db(&config().DB_DBNAME)
            .await
            .map_err(ModelManagerError::SetUseNSandDb)?;

        Ok(db)
    }

    /// Function that tries to connect to the bucket and creates it
    async fn connect_minio() -> Result<Bucket> {
        let bucket_name = &config().S3_BUCKET_NAME;

        let region = Region::Custom {
            region: config().S3_REGION.to_string(),
            endpoint: config().S3_ENDPOINT.to_string(),
        };

        //Init credentials, unwrap if cannot create default
        let creds = Credentials::new(
            Some(&config().S3_ACCESS_KEY),
            Some(&config().S3_SECRET_KEY),
            None,
            None,
            None,
        )
        .map_err(ModelManagerError::MinioCredentials)?;

        let mut bucket = Bucket::new(bucket_name, region.clone(), creds.clone())
            .map_err(ModelManagerError::NewBucket)?
            .with_path_style();

        if !bucket
            .exists()
            .await
            .map_err(ModelManagerError::BucketExists)?
        {
            bucket = Bucket::create_with_path_style(
                bucket_name,
                region,
                creds,
                BucketConfiguration::default(),
            )
            .await
            .map_err(ModelManagerError::CreateBucket)?
            .bucket;
        }

        Ok(bucket)
    }

    pub async fn upload(&self, file_name: &str, file: Bytes) -> anyhow::Result<()> {
        self.bucket.put_object(file_name, &file).await?;

        Ok(())
    }

    pub async fn download(&self, file_name: &str) -> anyhow::Result<Bytes> {
        let r = self.bucket.get_object(file_name).await?;

        Ok(Bytes::copy_from_slice(r.as_slice()))
    }

    pub fn db(&self) -> &Surreal<Client> {
        &self.db
    }

    pub fn bucket(&self) -> &Bucket {
        &self.bucket
    }
}
