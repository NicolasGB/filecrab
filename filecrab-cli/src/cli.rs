use std::io::{self, Write};

use anyhow::{bail, Ok};

use clap::{Parser, Subcommand};
use config::Config;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Parser)]
pub struct Cli {
    /// Which command are we runnig
    #[command(subcommand)]
    pub command: Command,

    #[clap(skip)]
    settings: Option<Settings>,

    #[clap(skip)]
    client: Client,
}

#[derive(Clone, Subcommand)]
pub enum Command {
    Upload {
        path: String,
        password: Option<String>,
    },
    Download,
    Copy,
    Paste,
}

#[derive(Serialize, Deserialize)]
pub struct Settings {
    api_key: String,
    url: String,
}

#[derive(Serialize, Deserialize)]
struct UploadResponse {
    pub id: String,
}

impl Cli {
    pub async fn run(mut self) -> anyhow::Result<()> {
        self.set_client();

        match self.command {
            Command::Upload {
                ref path,
                ref password,
            } => self.upload(path.clone(), password.clone()).await?,
            Command::Download => todo!(),
            Command::Copy => todo!(),
            Command::Paste => self.paste().await?,
        };

        Ok(())
    }
    fn set_client(&mut self) {
        self.client = reqwest::Client::new();
    }

    fn set_config(&mut self) -> anyhow::Result<()> {
        let home = match home::home_dir() {
            Some(path) => path.to_str().unwrap_or_default().to_owned(),
            None => bail!("Could not locate home path which is mandatory for filecrab"),
        };

        let raw_config = Config::builder()
            .add_source(config::File::with_name(&format!(
                "{home}/.config/filecrab/config.toml"
            )))
            .build()?;

        let app_settings = raw_config.try_deserialize::<Settings>()?;
        self.settings = Some(app_settings);
        Ok(())
    }

    async fn paste(mut self) -> anyhow::Result<()> {
        self.set_config()?;

        // Safe to unwrap as the previous function would have errored
        //TODO: 23/01/2024 - should still avoid to unwrap once the cli is done
        let Settings { api_key, url } = self.settings.unwrap();

        self.client
            .post(url)
            .header("filecrab-key", api_key)
            .send()
            .await?;

        Ok(())
    }

    async fn upload(&mut self, path: String, password: Option<String>) -> anyhow::Result<()> {
        self.set_config()?;

        // Safe to unwrap as the previous function would have errored
        //TODO: 23/01/2024 - should still avoid to unwrap once the cli is done
        let Settings { api_key, url } = self.settings.as_ref().unwrap();

        // Get the name of the file
        let file_name = path.rsplit('/').next().unwrap_or("").to_owned();
        // Read the file, should stream it
        let file = fs::read(path).await?;
        let part = reqwest::multipart::Part::bytes(file).file_name(file_name);
        let mut form = reqwest::multipart::Form::new().part("file", part);

        // If there's a password set it to the multipart
        if let Some(pwd) = password {
            form = form.text("password", pwd)
        }

        // Send the request
        let resp = self
            .client
            .post(format!("{url}/api/upload"))
            .header("filecrab-key", api_key)
            .multipart(form)
            .send()
            .await?
            .json::<UploadResponse>()
            .await?;

        println!("The id to share is the following: {}", resp.id);

        // Copy it to the clipboard
        let mut clipboard = arboard::Clipboard::new()?;
        clipboard.set_text(resp.id)?;
        println!("It has now been copied to your clipboard, share it before the program exits!");

        // Prompt the user to press Enter to exit
        println!("Press Enter to exit...");
        io::stdout().flush().unwrap();
        let mut buffer = String::new();
        io::stdin()
            .read_line(&mut buffer)
            .expect("Failed to read input");

        Ok(())
    }
}
