use argon2::Config;
use rand::Rng;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

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
    pub async fn create(mm: ModelManager, data: &mut AssetToCreate) -> Result<Asset> {
        let db = mm.db();

        if data.password.is_some() {
            data.password = data.password.take().map(|password| {
                // Get the config
                let config = Config::default();

                //Get the thread rng
                let mut rng = rand::thread_rng();
                let mut salt = [0u8; 32];

                // Use map to handle the result of try_fill
                rng.try_fill(&mut salt)
                    .ok()
                    .and_then(|_| argon2::hash_encoded(&password.into_bytes(), &salt, &config).ok())
                    .unwrap_or_default()
            });
            if data.password.is_none() {
                return Err(ModelManagerError::CouldNotHashPassword);
            }
        }

        let mut res: Vec<Asset> = db
            .create("asset")
            .content(data)
            .await
            .map_err(ModelManagerError::CreateAsset)?;

        let ass = res.pop();
        ass.ok_or_else(|| ModelManagerError::AssetNotFound)
    }
}
