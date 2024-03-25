use chrono::Utc;
use serde::{Deserialize, Serialize};
use surrealdb::sql::{Datetime, Thing};

use super::error::{ModelManagerError, Result};
use crate::{config::config, model::ModelManager};

#[derive(Serialize, Deserialize)]
pub struct Text {
    pub id: Thing,
    pub content: String,
    pub memo_id: String,
    expire: Datetime,
}

#[derive(Serialize, Deserialize)]
pub struct TextToCreate {
    pub content: String,
    #[serde(skip_deserializing)]
    memo_id: String,
    #[serde(skip_deserializing)]
    expire: Datetime,
}

impl Text {
    pub async fn create(mm: ModelManager, data: &mut TextToCreate) -> Result<Text> {
        let db = mm.db();
        // Set a memo_id
        data.memo_id = memorable_wordlist::snake_case(40);

        // Set expire
        let expire = Utc::now() + config().DEFAULT_EXPIRE_TIME;
        data.expire = expire.into();

        let res: Vec<Text> = db
            .create("text")
            .content(data)
            .await
            .map_err(ModelManagerError::CreateText)?;

        res.into_iter()
            .next()
            .ok_or_else(|| ModelManagerError::TextNotFound)
    }

    pub async fn read(mm: ModelManager, memo_id: String) -> Result<Text> {
        let db = mm.db();

        let res: Option<Text> = db
            .query("SELECT * FROM text WHERE memo_id == $memo_id LIMIT 1")
            .bind(("memo_id", memo_id))
            .await
            .map_err(ModelManagerError::SearchText)?
            .take(0)
            .map_err(ModelManagerError::TakeError)?;

        res.ok_or_else(|| ModelManagerError::TextNotFound)
    }

    pub async fn delete(mm: ModelManager, id: String) -> Result<()> {
        let db = mm.db();

        let _: Option<Text> = db
            .delete(("text", id))
            .await
            .map_err(ModelManagerError::DeleteText)?;

        Ok(())
    }

    pub async fn clean_text(mm: ModelManager) -> Result<()> {
        let db = mm.db();

        let now: Datetime = Utc::now().into();

        let _ = db
            .query("DELETE text WHERE expire <= $now")
            .bind(("now", &now))
            .await
            .map_err(ModelManagerError::DeleteText)?;

        Ok(())
    }
}
