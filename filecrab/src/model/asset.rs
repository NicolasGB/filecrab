use argon2::Config;
use rand::Rng;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;
use tokio::time::Instant;
use tracing::debug;

use super::error::{ModelManagerError, Result};
use crate::model::ModelManager;

#[derive(Clone, Deserialize)]
pub struct Asset {
    pub id: Thing,
    pub password: Option<String>,
    pub file_name: String,
}

#[derive(Clone, Serialize)]
pub struct AssetToCreate {
    pub password: Option<String>,
    pub file_name: String,
}

impl Asset {
    pub async fn create(mm: ModelManager, id: &str, data: &mut AssetToCreate) -> Result<Asset> {
        let db = mm.db();

        //Hash password if set
        if data.password.is_some() {
            let hash = data
                .password
                .take()
                .map(|password| {
                    // Get the config
                    let config = Config::default();

                    //Get the thread rng
                    let mut rng = rand::thread_rng();
                    let mut salt = [0u8; 32];

                    // Use map to handle the result of try_fill
                    rng.try_fill(&mut salt)
                        .ok()
                        .and_then(|_| {
                            argon2::hash_encoded(&password.into_bytes(), &salt, &config).ok()
                        })
                        .unwrap_or_default()
                })
                .ok_or(ModelManagerError::CouldNotHashPassword)?;

            data.password = Some(hash)
        }

        let res: Option<Asset> = db
            .create(("asset", id))
            .content(data)
            .await
            .map_err(ModelManagerError::CreateAsset)?;

        res.ok_or_else(|| ModelManagerError::AssetNotFound)
    }
}
