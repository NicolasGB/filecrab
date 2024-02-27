use std::{
    cmp::min,
    env,
    io::{self, Write},
};

use anyhow::{bail, Ok};

use clap::{Parser, Subcommand};
use config::Config;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{self, OpenOptions},
    io::{AsyncWriteExt, BufWriter},
};

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
    /// Uploads a file to filecrab.
    Upload {
        /// Path to the file to upload.
        #[arg(long, short)]
        path: String,
        /// A secret password to protect the file from being downloaded.
        #[arg(long, short)]
        secret: Option<String>,
    },
    /// Downloads the file given by the id returned by the upload command.
    Download {
        /// The memorable word list, shared with you.
        #[arg(long = "file", short = 'f')]
        id: String,
        #[arg(long, short)]
        /// Secret pass to access the file if this one is protected.
        secret: Option<String>,
        #[arg(long, short)]
        /// If you don't want to save it to the cwd, set a path to save the file to.
        path: Option<String>,
    },
    /// WIP
    Copy,
    /// WIP
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
                secret: ref password,
            } => self.upload(path.clone(), password.clone()).await?,
            Command::Download {
                ref id,
                secret: ref password,
                ref path,
            } => {
                self.download(id.clone(), password.clone(), path.clone())
                    .await?
            }
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

        println!("The id to share is the following:");
        println!("  {}", resp.id);

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

    async fn download(
        &mut self,
        id: String,
        password: Option<String>,
        path: Option<String>,
    ) -> anyhow::Result<()> {
        self.set_config()?;

        // Safe to unwrap as the previous function would have errored
        //TODO: 23/01/2024 - should still avoid to unwrap once the cli is done
        let Settings { api_key, url } = self.settings.as_ref().unwrap();

        // If there's a password set it to the multipart
        let mut query: Vec<(&str, &str)> = vec![("file", &id)];

        // If a password has been set add it to the query params
        let pwd: String;
        if let Some(password) = password {
            pwd = password.clone();
            query.push(("password", &pwd))
        }

        // Send the request
        let resp = self
            .client
            .get(format!("{url}/api/download"))
            .header("filecrab-key", api_key)
            .query(&query)
            .send()
            .await?;

        // Chech if there's been an error
        if !resp.status().is_success() {
            bail!(format!("{}", resp.status().to_string()))
        }

        // Get the filename from the headers
        let filename = resp
            .headers()
            .get("filecrab-file-name")
            .map(|x| x.to_str().unwrap_or_default().to_string());

        // Get either given path or cwd
        let cwd = if let Some(p) = path {
            p
        } else {
            env::current_dir()?.to_str().unwrap_or_default().to_string()
        };

        // Create file with the name of the asset
        //TODO: 30/01/2024 - Implement better error messaging here
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(format!("{}/{}", cwd, filename.clone().unwrap_or_default()))
            .await?;
        // Create the buffer
        let mut out = BufWriter::new(file);

        // Get the content lenght for the progressbar
        let total_size = resp.content_length().unwrap_or_default();

        // Init the progress bar
        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
        .progress_chars("#>-"));
        pb.set_message("Downloading file...");

        // Read the stream
        let mut stream = resp.bytes_stream();
        let mut downloaded: u64 = 0;
        while let Some(data) = stream.next().await {
            // Borrow checker magic
            let chunk = data?;
            tokio::io::copy(&mut chunk.as_ref(), &mut out).await?;

            // Calculate new position
            let new = min(downloaded + (chunk.len() as u64), total_size);
            downloaded = new;

            pb.set_position(new);
        }

        // Flush the contents to the file only once
        out.flush().await?;

        // Finish the progress bar
        pb.finish_with_message(format!(
            "The name of the downloaded element is: {}",
            filename.unwrap_or_default()
        ));

        Ok(())
    }
}
