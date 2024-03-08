pub mod asset;
mod error;
pub mod text;

use std::io;

pub use error::{ModelManagerError, Result};

use axum::{body::Bytes, BoxError};
use futures::{Stream, TryStreamExt};
use surrealdb::{
    engine::remote::ws::{Client, Ws},
    opt::auth::Root,
    Surreal,
};
use tokio_util::io::StreamReader;

use crate::config::config;
use s3::{creds::Credentials, request::ResponseDataStream, Bucket, BucketConfiguration, Region};

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
            .map_err(|err| ModelManagerError::SetUseNSandDb {
                ns: config().DB_NS.to_string(),
                db: config().DB_NS.to_string(),
                source: err,
            })?;

        // Create the assets table
        db.query("DEFINE TABLE asset")
            .await
            .map_err(ModelManagerError::CouldNotDefineTable)?;

        // Set the search index in memo_id asset column
        db.query("DEFINE INDEX fileMemoIdUnique ON TABLE asset COLUMNS memo_id UNIQUE")
            .await
            .map_err(ModelManagerError::CouldNotSetTableIndex)?;

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
        )?;

        let mut bucket = Bucket::new(bucket_name, region.clone(), creds.clone())?.with_path_style();

        if !bucket.exists().await? {
            bucket = Bucket::create_with_path_style(
                bucket_name,
                region,
                creds,
                BucketConfiguration::default(),
            )
            .await?
            .bucket;
        }

        Ok(bucket)
    }

    pub async fn upload<S, E>(&self, file_name: &str, stream: S) -> Result<()>
    where
        S: Stream<Item = std::result::Result<Bytes, E>>,
        E: Into<BoxError>,
    {
        async {
            //Convert the stream into an 'AsyncRead'
            let body_with_io_error =
                stream.map_err(|err| io::Error::new(io::ErrorKind::Other, err));
            let body_reader = StreamReader::new(body_with_io_error);
            futures::pin_mut!(body_reader);

            self.bucket
                .put_object_stream(&mut body_reader, file_name)
                .await
        }
        .await?;

        Ok(())
    }

    pub async fn download(&self, file_name: &str) -> Result<(ResponseDataStream, usize)> {
        // We get the head of the object to be able to access the size of it
        let (head, _) = self.bucket.head_object(file_name).await?;

        // We get the object stream
        let r = self.bucket.get_object_stream(file_name).await?;
        // We return the stream and the content length
        Ok((r, head.content_length.unwrap_or_default() as usize))
    }

    pub fn db(&self) -> &Surreal<Client> {
        &self.db
    }

    pub async fn delete_files(&self, file_names: Vec<String>) -> Result<()> {
        for name in file_names.iter() {
            self.bucket.delete_object(name).await?;
        }

        Ok(())
    }
}