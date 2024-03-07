use std::io::{Read, Write};

use age::secrecy::Secret;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

use super::error::{ModelManagerError, Result};
use crate::model::ModelManager;

#[derive(Serialize, Deserialize)]
pub struct Text {
    pub id: Thing,
    pub content: String,
}

#[derive(Serialize, Deserialize)]
pub struct TextToCreate {
    pub content: String,
    #[serde(skip_serializing)]
    pub password: String,
}

impl Text {
    pub async fn create(mm: ModelManager, data: &mut TextToCreate) -> Result<Text> {
        data.content = {
            let encryptor =
                age::Encryptor::with_user_passphrase(Secret::new(data.password.clone()));

            let mut encrypted = vec![];
            let mut writer = encryptor.wrap_output(&mut encrypted)?;
            writer.write_all(data.content.as_bytes())?;
            writer.finish()?;

            // Encode to hex
            hex::encode_upper(encrypted)
        };

        let db = mm.db();

        let res: Vec<Text> = db
            .create("text")
            .content(data)
            .await
            .map_err(ModelManagerError::CreateText)?;

        res.into_iter()
            .next()
            .ok_or_else(|| ModelManagerError::TextNotFound)
    }

    pub async fn read(mm: ModelManager, id: String, password: String) -> Result<Text> {
        let db = mm.db();

        let mut res: Option<Text> = db
            .select(("text", id))
            .await
            .map_err(ModelManagerError::SearchText)?;

        res = if let Some(mut text) = res {
            let content = hex::decode(text.content)?;
            let decryptor = match age::Decryptor::new(&content[..])? {
                age::Decryptor::Passphrase(d) => d,
                _ => unreachable!(),
            };

            let mut decrypted = vec![];
            let mut reader = decryptor.decrypt(&Secret::new(password), None)?;

            reader.read_to_end(&mut decrypted)?;
            text.content = String::from_utf8_lossy(decrypted.as_ref()).to_string();
            Some(text)
        } else {
            None
        };

        //TODO: 09/02/2024 - Implement deletion after retrieving
        res.ok_or_else(|| ModelManagerError::TextNotFound)
    }
}
