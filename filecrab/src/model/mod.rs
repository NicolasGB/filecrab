use axum::body::Bytes;
use tracing::debug;

use crate::config::config;
use s3::{creds::Credentials, Bucket, BucketConfiguration, Region};

#[derive(Debug, Clone)]
pub struct ModelManager {
    //Bucket is clonable as its references are behind an arc
    bucket: Bucket,
}

impl ModelManager {
    pub async fn new() -> Self {
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
        .unwrap();

        let mut bucket = Bucket::new(bucket_name, region.clone(), creds.clone())
            .unwrap()
            .with_path_style();

        if !bucket.exists().await.unwrap() {
            bucket = Bucket::create_with_path_style(
                bucket_name,
                region,
                creds,
                BucketConfiguration::default(),
            )
            .await
            .unwrap()
            .bucket;
        }

        ModelManager { bucket }
    }

    pub async fn upload(&self, file_name: &str, file: Bytes) -> anyhow::Result<()> {
        self.bucket.put_object(file_name, &file).await?;

        Ok(())
    }

    pub async fn download(&self, file_name: &str) -> anyhow::Result<Bytes> {
        let r = self.bucket.get_object(file_name).await?;

        Ok(Bytes::copy_from_slice(r.as_slice()))
    }
}
