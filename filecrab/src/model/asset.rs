use std::ops::Add;

use argon2::Config;
use chrono::{prelude::*, Days, Duration};
use rand::Rng;
use serde::{Deserialize, Serialize};
use surrealdb::sql::{Datetime, Thing};
use tracing::debug;

use super::error::{ModelManagerError, Result};
use crate::{config::config, model::ModelManager};

#[derive(Clone, Deserialize)]
pub struct Asset {
    pub id: Thing,
    pub password: Option<String>,
    pub file_name: String,
}

#[derive(Clone, Serialize, Debug)]
pub struct AssetToCreate {
    pub password: Option<String>,
    pub file_name: String,
    pub expire: Option<Datetime>,
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

        debug!("{data:?}");

        // Add an expire time if it's not set
        if data.expire.is_none() {
            // If nothing is set default to the config's default expire time
            let now = Utc::now();
            let exp = now.add(Duration::hours(config().DEFAULT_EXPIRE_TIME as i64));
            data.expire = Some(exp.into());
        }

        let res: Option<Asset> = db
            .create(("asset", id))
            .content(data)
            .await
            .map_err(ModelManagerError::CreateAsset)?;

        res.ok_or_else(|| ModelManagerError::AssetNotFound)
    }
}
