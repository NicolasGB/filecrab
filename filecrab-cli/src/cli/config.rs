use crate::{error::Error, Result};
use std::{mem, path::PathBuf, vec};

use inquire::{validator::Validation, Confirm, CustomUserError, InquireError, Select, Text};
use serde::{Deserialize, Serialize};
use tokio::fs;

const CONFIG_PATH: &str = "filecrab/config.toml";

/// Represents the CLI config.
#[derive(Deserialize, Serialize, Default, Clone)]
pub(super) struct Config {
    pub(super) active: Instance,
    pub(super) others: Option<Vec<Instance>>,
}

/// Represents a filecrab instance.
#[derive(Deserialize, Serialize, Default, Clone)]
pub(super) struct Instance {
    pub(super) name: String,
    pub(super) url: String,
    pub(super) api_key: String,
}

/// Gives the original command and a pathbuf, used to create filecrab configs.
pub enum CommandAndPath<'a> {
    Init(&'a PathBuf),
    Other(&'a PathBuf),
}

impl Config {
    /// Returns the active instance
    pub(super) fn get_active_instance(&self) -> &Instance {
        &self.active
    }

    /// Loads the config.
    pub(super) async fn load_config() -> Result<Config> {
        // Builds the path to the config file.
        let config_path = match dirs::config_dir() {
            Some(config_dir) => config_dir.join(CONFIG_PATH),
            None => return Err(Error::ConfigNotFound),
        };

        // Prompts the user to set the config if it does not exist.
        if !config_path.exists() {
            Config::prompt_new_config(CommandAndPath::Other(&config_path)).await?;
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
    async fn prompt_new_config(command_and_path: CommandAndPath<'_>) -> Result<()> {
        let path = match command_and_path {
            CommandAndPath::Init(path) => {
                println!("Initializing config:");
                path
            }
            CommandAndPath::Other(path) => {
                println!("The config file is not set, we're going to create it:");
                path
            }
        };

        // Get the new instance
        let instance = Config::prompt_instance_input(None)?;

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

    /// Writes the config to the given path
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

    /// Prompts the user to get the input of a new instance
    ///
    /// INFO: As of `now prompt_instance_input` needs it's own version of config (if passed) due to the
    /// 'static lifetime of the with_validator in inquire. Maybe one day this will be fixed in the
    /// lib, an issue has been opened.
    fn prompt_instance_input(config: Option<Config>) -> Result<Instance> {
        let instance_name = Text::new("What's the filecrab instance's name?")
            .with_validator(
                move |val: &str| -> std::result::Result<Validation, CustomUserError> {
                    // So we make sure we don't consume the config here, consuming the config
                    // appears to fail the 'static annotation of the closure
                    if let Some(conf) = &config {
                        // If either the active config or the others already have the given name,
                        // refuse the validation
                        if conf.active.name == val
                            || conf
                                .others
                                .as_ref()
                                .is_some_and(|others| others.iter().any(|i| i.name == val))
                        {
                            return Ok(Validation::Invalid(
                                "This instance name is already in use.".into(),
                            ));
                        }
                    }
                    Ok(Validation::Valid)
                },
            )
            .prompt()
            .map_err(|err| match err {
                InquireError::OperationCanceled | InquireError::OperationInterrupted => {
                    Error::UserCancel
                }
                _ => err.into(),
            })?
            .to_string();

        // Ask the user for the url
        let url = Text::new("Enter the complete URL of your filecrab:")
            .with_initial_value("https://")
            .with_validator(|val: &str| {
                if !val.starts_with("http://") && !val.starts_with("https://") {
                    return Ok(Validation::Invalid(
                        "The given url is missing the `http(s)://` prefix.".into(),
                    ));
                }
                Ok(Validation::Valid)
            })
            .with_help_message(
                "The `http(s)` prefix is mandatory! You should also set the port if needed.",
            )
            .prompt()
            .map_err(|err| match err {
                InquireError::OperationCanceled | InquireError::OperationInterrupted => {
                    Error::UserCancel
                }
                _ => err.into(),
            })?;

        let url = url.trim().to_string();

        // Reads the API key from the stdin.
        let api_key = Text::new("Enter the API key:").prompt()?.trim().to_string();

        Ok(Instance {
            name: instance_name,
            url,
            api_key,
        })
    }

    pub(super) async fn switch_instance(&mut self) -> Result {
        let others = self.others.as_mut().ok_or(Error::NoOtherInstances)?;

        // Collect all the names
        let names = others.iter().map(|i| i.name.as_str()).collect();

        // Ask the user which instance to activate
        let new_name = Select::new("Which Filecrab instance do you want to activate?", names)
            .prompt()
            .map_err(|err| match err {
                InquireError::OperationCanceled | InquireError::OperationInterrupted => {
                    Error::UserCancel
                }
                _ => err.into(),
            })?
            .to_string();

        // Find the instance to switch to
        if let Some(new) = others.iter_mut().find(|i| i.name == new_name) {
            mem::swap(&mut self.active, new);
        }

        // Create the path
        let path = match dirs::config_dir() {
            Some(config_dir) => config_dir.join(CONFIG_PATH),
            None => return Err(Error::ConfigNotFound),
        };

        // Write the config
        Config::write_config(&path, self).await?;

        println!("Successfully switched to `{new_name}`.");

        Ok(())
    }

    pub(super) async fn add(&mut self) -> Result {
        // Prompt the user and get the new instance
        println!("Adding a new instance:");
        let new_instance = Config::prompt_instance_input(Some(self.clone()))?;
        let new_name = new_instance.name.clone();

        // Push the instance to the others
        match self.others.as_mut() {
            Some(others) => {
                // If one already exists
                if others.iter().any(|i| i.name == new_name) {
                    return Err(Error::DuplicateInstanceName(new_name.clone()));
                }

                others.push(new_instance)
            }
            None => self.others = Some(vec![new_instance]),
        }

        // Prompt the user if he want's to switch it as active
        let ans = Confirm::new("Do you want to set it as the active instance?")
            .with_default(false)
            .prompt()?;

        // If yes, switch it
        if ans {
            // We unwrap as this should never fail because it has just been added
            let others = self.others.as_mut().unwrap();

            // Find the instance to switch to
            if let Some(new) = others.iter_mut().find(|i| i.name == new_name) {
                mem::swap(&mut self.active, new);
            }
        }

        // Create the path
        let path = match dirs::config_dir() {
            Some(config_dir) => config_dir.join(CONFIG_PATH),
            None => return Err(Error::ConfigNotFound),
        };

        // Write the config
        Config::write_config(&path, self).await?;

        if ans {
            println!("Successfully added `{new_name}` and switched it as active.")
        } else {
            println!("Successfully added `{new_name}`.")
        }

        Ok(())
    }

    pub(super) async fn remove(&mut self) -> Result {
        // Build a list to select from
        let mut names = vec![self.active.name.clone()];

        // If there are other instances push also it's names
        if let Some(others) = self.others.as_ref() {
            others.iter().for_each(|i| names.push(i.name.clone()))
        }

        // Ask the user which instance to remove
        let name_to_remove = Select::new("Which Filecrab instance do you want to remove?", names)
            .prompt()
            .map_err(|err| match err {
                InquireError::OperationCanceled | InquireError::OperationInterrupted => {
                    Error::UserCancel
                }
                _ => err.into(),
            })?
            .to_string();

        // Make sure the user want's to remove it
        if !Confirm::new("Are you sure?").with_default(false).with_help_message("Removing the active instance can have 2 consequences, if there are no more instances the config file is deleted. Otherwise the first instance in `others` is swapped as active.").prompt()? {
            println!("Exited without removing any instance.");
            return Ok(());
        }

        // If the one being deleted is the active one, either swap it with another or remove the
        // config file
        if name_to_remove == self.active.name.as_ref() {
            match self.others.as_mut() {
                // Remove the first of the others and place it as active
                Some(others) => {
                    let mut new_active = others.remove(0);
                    mem::swap(&mut self.active, &mut new_active);
                }
                None => self.delete_config_file().await?,
            }
        } else {
            // Unwrapping is safe as if the selected name is not the active it for sure exists in
            // the others
            self.others
                .as_mut()
                .unwrap()
                .retain(|i| i.name != name_to_remove);
        }

        // If there are no remaining instances, set the others as None
        if self.others.as_ref().is_some_and(|i| i.is_empty()) {
            self.others = None;
        }

        // Create the path
        let path = match dirs::config_dir() {
            Some(config_dir) => config_dir.join(CONFIG_PATH),
            None => return Err(Error::ConfigNotFound),
        };

        // Write the config
        Config::write_config(&path, self).await?;

        Ok(())
    }

    pub(super) async fn delete_config_file(&self) -> Result {
        // Create the path
        let path = match dirs::config_dir() {
            Some(config_dir) => config_dir.join(CONFIG_PATH),
            None => return Err(Error::ConfigNotFound),
        };

        fs::remove_file(path).await.map_err(Error::RemoveConfig)?;

        Ok(())
    }

    pub(super) async fn init(&self) -> Result {
        let path = match dirs::config_dir() {
            Some(config_dir) => config_dir.join(CONFIG_PATH),
            None => return Err(Error::ConfigNotFound),
        };

        let exists = fs::try_exists(&path).await;
        // if exists is ok then return early
        if exists.as_ref().is_ok_and(|val| *val) {
            return Err(Error::ConfigExists);
        } else if exists.is_err() {
            exists.map_err(Error::FindConfig)?;
        }

        // If it doesn't exists prompt the user
        Config::prompt_new_config(CommandAndPath::Init(&path)).await?;

        Ok(())
    }
}
