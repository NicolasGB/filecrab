use crate::{error::Error, Result};
use std::path::PathBuf;

use inquire::{validator::Validation, Select, Text};
use serde::{Deserialize, Serialize};
use tokio::fs;

const CONFIG_PATH: &str = "filecrab/config.toml";

/// Represents the CLI config.
#[derive(Deserialize, Serialize, Default)]
pub(crate) struct Config {
    pub(crate) active: Instance,
    pub(crate) others: Option<Vec<Instance>>,
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub(crate) struct Instance {
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) api_key: String,
}

impl Config {
    /// Returns the active instance
    pub(crate) fn get_active_instance(&self) -> &Instance {
        &self.active
    }

    /// Loads the config.
    pub(crate) async fn load_config() -> Result<Config> {
        // Builds the path to the config file.
        let config_path = match dirs::config_dir() {
            Some(config_dir) => config_dir.join(CONFIG_PATH),
            None => return Err(Error::ConfigNotFound),
        };

        // Prompts the user to set the config if it does not exist.
        if !config_path.exists() {
            Config::prompt_new_config(&config_path).await?;
        }

        // Deserializes the config.
        let config = toml::from_str(&fs::read_to_string(&config_path).await.map_err(|err| {
            Error::ReadFile {
                path: format!("{}", config_path.display()),
                source: err,
            }
        })?)
        .map_err(Error::ParseToml)?;

        Ok(config)
    }

    /// Prompts the user to set the initial config and saves it.
    async fn prompt_new_config(path: &PathBuf) -> Result<()> {
        // Reads the URL from the stdin.
        println!("The config file is not set, we're going to create it.");
        println!();

        // Get the new instance
        let instance = Config::prompt_instance_input().await?;

        // Build new config struct
        let config = Config {
            active: instance,
            others: None,
        };

        Config::write_config(path, &config).await?;

        // Prints the completion message.
        println!();
        println!("Thanks, your file has been written in {path:?}. You can modify it manually.");
        println!("Enjoy pinching files and text! BLAZINGLY FAST!");
        println!();
        Ok(())
    }

    async fn write_config(path: &PathBuf, config: &Config) -> Result {
        // Builds the config and writes it to the file.
        let parent = match path.parent() {
            Some(parent) => parent,
            None => return Err(Error::NoParentDir),
        };

        // Create dir all if needed
        fs::create_dir_all(parent)
            .await
            .map_err(Error::CreateConfigDir)?;
        fs::write(
            path,
            &toml::to_string(config).map_err(Error::SerializeToml)?,
        )
        .await
        .map_err(|err| Error::WriteFile {
            path: format!("{}", path.display()),
            source: err,
        })
    }

    // Prompts the user to get the input of a new instance
    async fn prompt_instance_input() -> Result<Instance> {
        let instance_name = Text::new("What's the filecrab instance's name?")
            .prompt()?
            .to_string();

        // Ask the user for the url
        let url = Text::new("Enter the complete URL of your filecrab:")
            .with_initial_value("https://")
            .with_validator(|val: &str| {
                if !val.contains("http://") && !val.contains("https://") {
                    return Ok(Validation::Invalid(
                        "The given url is missing the `http(s)://` prefix.".into(),
                    ));
                }
                Ok(Validation::Valid)
            })
            .with_help_message(
                "The `http(s)` prefix is mandatory! You should also set the port if needed.",
            )
            .prompt()?;

        let url = url.trim().to_string();

        // Reads the API key from the stdin.
        let api_key = Text::new("Enter the API key:").prompt()?.trim().to_string();

        Ok(Instance {
            name: instance_name,
            url,
            api_key,
        })
    }

    pub(crate) async fn switch_instance(&mut self) -> Result {
        let others = self.others.as_mut().ok_or(Error::NoOtherInstances)?;

        // Collect all the names
        let names = others.iter().map(|i| i.name.as_str()).collect();

        // Ask the user which instance to activate
        let new_name = Select::new("Which Filecrab instance do you want to activate?", names)
            .prompt()?
            .to_string();

        // Find the instance to switch to
        let pos = others.iter().position(|i| i.name == new_name);

        // If no instance found return (should not happen as it's a prebuilt list)
        if pos.is_none() {
            return Err(Error::InstanceNotFound);
        }

        // Save the old active
        let old = self.active.clone();

        // Set the new active
        self.active = others.swap_remove(pos.unwrap());

        // push the old active to the others
        others.push(old);

        // Create the path
        let path = match dirs::config_dir() {
            Some(config_dir) => config_dir.join(CONFIG_PATH),
            None => return Err(Error::ConfigNotFound),
        };

        // Write the config
        Config::write_config(&path, self).await?;

        println!("Successfully switched to `{new_name}`");

        Ok(())
    }
}
