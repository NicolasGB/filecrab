use std::ops::Add;

use argon2::Config;
use chrono::{prelude::*, Duration};
use rand::Rng;
use serde::{Deserialize, Serialize};
use surrealdb::sql::{Datetime, Thing};
use tracing::info;

use super::error::{ModelManagerError, Result};
use crate::{config::config, model::ModelManager};

#[derive(Clone, Deserialize)]
pub struct Asset {
    pub id: Thing,
    pub password: Option<String>,
    pub file_name: String,
    pub memo_id: String,
    pub expire: Option<Datetime>,
}

#[derive(Clone, Serialize, Debug)]
pub struct AssetToCreate {
    pub password: Option<String>,
    pub file_name: String,
    pub expire: Option<Datetime>,
    pub memo_id: Option<String>,
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

        // Add an expire time if it's not set
        if data.expire.is_none() {
            // If nothing is set default to the config's default expire time
            let now = Utc::now();
            let exp = now.add(Duration::hours(config().DEFAULT_EXPIRE_TIME as i64));
            data.expire = Some(exp.into());
        }

        //Set the autogenerated memo id
        data.memo_id = Some(memorable_wordlist::snake_case(40));

        let res: Option<Asset> = db
            .create(("asset", id))
            .content(data)
            .await
            .map_err(ModelManagerError::CreateAsset)?;

        res.ok_or_else(|| ModelManagerError::AssetNotFound)
    }

    pub async fn read_by_memo_id(mm: ModelManager, memo_id: &str) -> Result<Asset> {
        let db = mm.db();

        let res: Option<Asset> = db
            .query("SELECT * FROM asset WHERE memo_id == $memo_id LIMIT 1")
            .bind(("memo_id", memo_id))
            .await
            .map_err(ModelManagerError::SearchAsset)?
            .take(0)
            .map_err(ModelManagerError::TakeError)?;

        res.ok_or_else(|| ModelManagerError::AssetNotFound)
    }

    pub async fn clean_assets(mm: ModelManager) -> Result<()> {
        let db = mm.db();

        let now: Datetime = Utc::now().into();

        let _res = db
            .query("DELETE asset WHERE expire <= $now")
            .bind(("now", now))
            .await
            .map_err(ModelManagerError::DeleteAsset)?;

        Ok(())
    }
}

impl Asset {
    /// Checks if the given password matches the one from the asset if this has one
    pub fn check_password(&self, pwd: String) -> Result<bool> {
        if let Some(hash) = &self.password {
            return argon2::verify_encoded(hash, &pwd.into_bytes())
                .map_err(|_| ModelManagerError::InvalidPasswod);
        }
        Ok(true)
    }
}
