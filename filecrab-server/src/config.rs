use std::{env, sync::OnceLock};

use chrono::TimeDelta;

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

    pub MAXIMUM_FILE_SIZE: usize,
    pub DEFAULT_EXPIRE_TIME: TimeDelta,

    pub DB_HOST_OR_PATH: String,
    pub DB_NS: String,
    pub DB_DBNAME: String,
    pub DB_USER: String,
    pub DB_PASSWORD: String,

    pub API_KEY: String,
}

impl Config {
    pub fn load_from_env() -> Result<Config> {
        Ok(Config {
            S3_BUCKET_NAME: get_env("S3_BUCKET_NAME")?,
            S3_REGION: get_env("S3_REGION")?,
            S3_ENDPOINT: get_env("S3_ENDPOINT")?,
            S3_ACCESS_KEY: get_env("S3_ACCESS_KEY")?,
            S3_SECRET_KEY: get_env("S3_SECRET_KEY")?,

            MAXIMUM_FILE_SIZE: get_env("MAXIMUM_FILE_SIZE")?.parse().map_err(|err| {
                eprintln!("{err}");
                Error::InvalidEnvType("MAXIMUM_FILE_SIZE")
            })?,
            DEFAULT_EXPIRE_TIME: convert_to_hours(get_env("DEFAULT_EXPIRE_TIME")?)?,

            DB_HOST_OR_PATH: get_env("DB_HOST_OR_PATH")?,
            DB_NS: get_env("DB_NS")?,
            DB_DBNAME: get_env("DB_DBNAME")?,
            DB_USER: get_env("DB_USER")?,
            DB_PASSWORD: get_env("DB_PASSWORD")?,
            API_KEY: get_env("API_KEY")?,
        })
    }
}

fn get_env(name: &'static str) -> Result<String> {
    env::var(name).map_err(|_| Error::ConfigMissingEnv(name))
}

fn convert_to_hours(time: String) -> Result<TimeDelta> {
    //Convert string to i64
    let hours: i64 = time.parse().map_err(|_| Error::CouldNotParseInt(time))?;

    TimeDelta::try_hours(hours).ok_or(Error::CouldNotConvertToHours)
}
