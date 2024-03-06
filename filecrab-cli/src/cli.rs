use std::{
    cmp::min,
    env,
    io::{self, Read, Write},
    path::Path,
    vec,
};

use age::secrecy::Secret;
use anyhow::{bail, Ok};

use clap::{Parser, Subcommand};
use config::Config;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{self, OpenOptions},
    io::AsyncWriteExt,
};

#[derive(Parser)]
pub struct Cli {
    /// Which command are we running
    #[command(subcommand)]
    pub command: Command,

    #[clap(skip)]
    settings: Settings,

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
    /// Copies the remote text, which will be decrypted, to the clipboard.
    Copy {
        #[arg(long, short)]
        id: String,
        #[arg(long, short)]
        secret: String,
    },
    /// Pastes the given text and uploads it to filecrab, the text will be encrypted.
    Paste {
        #[arg(long, short)]
        content: String,
        #[arg(long, short)]
        secret: String,
    },
}

#[derive(Serialize, Deserialize, Default)]
pub struct Settings {
    api_key: String,
    url: String,
}

#[derive(Deserialize)]
struct UploadResponse {
    pub id: String,
}

#[derive(Serialize)]
struct PasteBody {
    content: String,
    password: String,
}

#[derive(Deserialize)]
struct PasteResponse {
    id: String,
}

#[derive(Deserialize)]
struct CopyResponse {
    content: String,
}

impl Cli {
    pub async fn run(mut self) -> anyhow::Result<()> {
        self.client = reqwest::Client::new();

        match &self.command {
            Command::Upload {
                path,
                secret: password,
            } => self.upload(path.clone(), password.clone()).await,
            Command::Download {
                id,
                secret: password,
                path,
            } => {
                self.download(id.clone(), password.clone(), path.clone())
                    .await
            }
            Command::Copy {
                id,
                secret: password,
            } => self.copy(id.clone(), password.clone()).await,
            Command::Paste {
                content,
                secret: password,
            } => self.paste(content.clone(), password.clone()).await,
        }
    }

    async fn set_config(&mut self) -> anyhow::Result<()> {
        let home = match home::home_dir() {
            Some(path) => path.to_str().unwrap_or_default().to_owned(),
            None => bail!("Could not locate home path which is mandatory for filecrab"),
        };

        let path = format!("{home}/.config/filecrab/config.toml");

        // Check if the config file exists, if not create it and prompt the user in the process
        if !Path::new(&path).exists() {
            self.init_config(&path).await?;
        }

        // Read the config
        let raw_config = Config::builder()
            .add_source(config::File::with_name(&path))
            .build()?;

        let app_settings = raw_config.try_deserialize::<Settings>()?;
        self.settings = app_settings;
        Ok(())
    }

    async fn init_config(&self, path: &String) -> anyhow::Result<()> {
        // Prompt the user to press Enter to exit
        println!("The config file is not set, we're going to create it.");
        println!(
            "Please enter the complete url of your filecrab (ex: https://my-filecrab-instance.com):"
        );
        io::stdout().flush().unwrap();
        let mut url = String::new();
        io::stdin()
            .read_line(&mut url)
            .expect("Failed to read filecrab url");

        println!("Enter the api-key to be used:");
        io::stdout().flush().unwrap();
        let mut api_key = String::new();
        io::stdin()
            .read_line(&mut api_key)
            .expect("Failed to read api-key");

        // Build settings and write to file
        let mut settings = toml::map::Map::new();
        settings.insert("url".into(), url.trim().into());
        settings.insert("api_key".into(), api_key.trim().into());

        let toml_string = toml::to_string(&settings)?;
        fs::write(path, &toml_string).await?;

        println!();
        println!("Thanks, your file has been set in \"~/.config/filecrab/config.toml\". You can modify it manually if the parameters change in the future.");
        println!("Enjoy pinching files and text! BLAZINLGY FAST");
        println!();

        Ok(())
    }

    async fn paste(&mut self, content: String, password: String) -> anyhow::Result<()> {
        self.set_config().await?;

        // Safe to unwrap as the previous function would have errored
        let Settings { api_key, url } = &self.settings;

        let body = PasteBody { content, password };
        let resp: PasteResponse = self
            .client
            .post(format!("{url}/api/paste"))
            .json(&body)
            .header("filecrab-key", api_key)
            .send()
            .await?
            .json()
            .await?;

        self.copy_to_clipboard(resp.id)?;

        Ok(())
    }

    async fn copy(&mut self, id: String, password: String) -> anyhow::Result<()> {
        self.set_config().await?;

        // Safe to unwrap as the previous function would have errored
        let Settings { api_key, url } = &self.settings;

        // build the query params
        let query = vec![("id", id), ("password", password)];

        let resp: CopyResponse = self
            .client
            .get(format!("{url}/api/copy"))
            .query(&query)
            .header("filecrab-key", api_key)
            .send()
            .await?
            .json()
            .await?;

        // Set the content to the keyboard
        self.copy_to_clipboard(resp.content)?;

        Ok(())
    }

    async fn upload(&mut self, path: String, password: Option<String>) -> anyhow::Result<()> {
        self.set_config().await?;

        // Safe to unwrap as the previous function would have errored
        let Settings { api_key, url } = &self.settings;

        // Get the name of the file
        let file_name = path.rsplit('/').next().unwrap_or("").to_owned();
        // Read the file, should stream it
        let mut file = fs::read(path).await?;

        // Initialize the form
        let mut form = reqwest::multipart::Form::new();

        // If there's a password set it to the multipart and encrypt the file
        if let Some(pwd) = password {
            // Set the file password
            form = form.text("password", pwd.clone());
            // Encrypt the file using apassphrase...
            file = {
                let encryptor = age::Encryptor::with_user_passphrase(Secret::new(pwd));

                let mut encrypted = vec![];
                let mut writer = encryptor.wrap_output(&mut encrypted)?;
                writer.write_all(file.as_slice())?;
                writer.finish()?;

                encrypted
            };
        }

        let part = reqwest::multipart::Part::bytes(file).file_name(file_name);
        form = form.part("file", part);

        // Send the request
        let resp: UploadResponse = self
            .client
            .post(format!("{url}/api/upload"))
            .header("filecrab-key", api_key)
            .multipart(form)
            .send()
            .await?
            .json()
            .await?;

        println!("The id to share is the following:");
        println!("  {}", resp.id);

        self.copy_to_clipboard(resp.id)?;

        Ok(())
    }

    async fn download(
        &mut self,
        id: String,
        password: Option<String>,
        path: Option<String>,
    ) -> anyhow::Result<()> {
        self.set_config().await?;

        // Safe to unwrap as the previous function would have errored
        let Settings { api_key, url } = &self.settings;

        // If there's a password set it to the multipart
        let mut query: Vec<(&str, &str)> = vec![("file", &id)];

        // If a password has been set add it to the query params
        let mut pwd: String = "".to_string();
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

        // Check if there's been an error
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
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(format!("{}/{}", cwd, filename.clone().unwrap_or_default()))
            .await?;

        // Get the content length for the progressbar
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
        // Create the buffer
        let mut buf: Vec<u8> = vec![];
        while let Some(data) = stream.next().await {
            // Borrow checker magic
            let chunk = data?;
            tokio::io::copy(&mut chunk.as_ref(), &mut buf).await?;
            // Calculate new position
            let new = min(downloaded + (chunk.len() as u64), total_size);
            downloaded = new;

            pb.set_position(new);
        }

        let decrypted;
        let data_to_write = if !pwd.is_empty() {
            decrypted = {
                let decryptor = match age::Decryptor::new(&buf[..]).unwrap() {
                    age::Decryptor::Passphrase(d) => d,
                    _ => unreachable!(),
                };

                let mut decrypted = vec![];
                let mut reader = decryptor.decrypt(&Secret::new(pwd.to_owned()), None)?;

                reader.read_to_end(&mut decrypted)?;

                decrypted
            };
            decrypted.as_ref()
        } else {
            buf.as_ref()
        };

        file.write_all(data_to_write).await?;

        // Finish the progress bar
        pb.finish_with_message(format!(
            "The name of the downloaded element is: {}",
            filename.unwrap_or_default()
        ));

        Ok(())
    }

    /// Sets the text to the keyboard and waits for the user to <CR> before returning. This will
    /// allow the user to copy and paste the contents as long as they wish holding the program's
    /// exit.
    fn copy_to_clipboard(&self, text: String) -> anyhow::Result<()> {
        // Copy it to the clipboard
        let mut clipboard = arboard::Clipboard::new()?;
        clipboard.set_text(text)?;
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
