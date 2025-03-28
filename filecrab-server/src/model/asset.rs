use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use surrealdb::sql::{Datetime, Thing};

use super::error::{ModelManagerError, Result};
use crate::{config::config, model::ModelManager};

#[derive(Clone, Deserialize)]
pub struct Asset {
    pub id: Thing,
    pub file_name: String,
    pub memo_id: String,
}

#[derive(Clone, Serialize, Debug)]
pub struct AssetToCreate {
    pub encrypted: bool,
    pub file_name: String,
    pub expire: Option<Datetime>,
    pub memo_id: Option<String>,
}

impl Asset {
    pub async fn create(mm: ModelManager, id: &str, mut data: AssetToCreate) -> Result<Asset> {
        let db = mm.db();

        // Add an expire time if it's not set
        if data.expire.is_none() {
            // If nothing is set default to the config's default expire time
            let exp = Utc::now() + config().DEFAULT_EXPIRE_TIME;
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
            .query("SELECT * FROM asset WHERE memo_id = $memo_id LIMIT 1")
            .bind(("memo_id", memo_id.to_string()))
            .await
            .map_err(ModelManagerError::SearchAsset)?
            .take(0)
            .map_err(ModelManagerError::TakeError)?;

        res.ok_or_else(|| ModelManagerError::AssetNotFound)
    }

    pub async fn clean_assets(mm: ModelManager) -> Result<Vec<String>> {
        let db = mm.db();

        let now: Datetime = Utc::now().into();

        // Get all deletable assets
        let res: Vec<Thing> = db
            .query("SELECT id FROM asset WHERE expire <= $now")
            .bind(("now", now.clone()))
            .await
            .map_err(ModelManagerError::DeleteAsset)?
            .take((0, "id"))
            .map_err(ModelManagerError::TakeError)?;

        // Collect only the id out of the things
        let res = res.into_iter().map(|v| v.id.to_string()).collect();

        let _ = db
            .query("DELETE asset WHERE expire <= $now")
            .bind(("now", now.clone()))
            .await
            .map_err(ModelManagerError::DeleteAsset)?;

        Ok(res)
    }
}
