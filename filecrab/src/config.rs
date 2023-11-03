use std::{env, sync::OnceLock};

use crate::{Error, Result};

//We want to get the config only once
pub fn config() -> &'static Config {
    //First the instance is empty
    static INSTANCE: OnceLock<Config> = OnceLock::new();

    INSTANCE.get_or_init(|| {
        Config::load_from_env()
            .unwrap_or_else(|err| panic!("FATAL WHILE LOADING CONFIG. Cause: {err:?}"))
    })
}

#[allow(non_snake_case)]
pub struct Config {
    pub S3_BUCKET_NAME: String,
    pub S3_REGION: String,
    pub S3_ENDPOINT: String,
    pub S3_ACCESS_KEY: String,
    pub S3_SECRET_KEY: String,
}

impl Config {
    pub fn load_from_env() -> Result<Config> {
        Ok(Config {
            S3_BUCKET_NAME: get_env("S3_BUCKET_NAME")?,
            S3_REGION: get_env("S3_REGION")?,
            S3_ENDPOINT: get_env("S3_ENDPOINT")?,
            S3_ACCESS_KEY: get_env("S3_ACCESS_KEY")?,
            S3_SECRET_KEY: get_env("S3_SECRET_KEY")?,
        })
    }
}

fn get_env(name: &'static str) -> Result<String> {
    env::var(name).map_err(|_| Error::ConfigMissingEnv(name))
}
