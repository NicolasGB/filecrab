use age::{secrecy::Secret, Decryptor, Encryptor};
use anyhow::{bail, Ok, Result};
use arboard::Clipboard;
use clap::{Parser, Subcommand};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{
    multipart::{Form, Part},
    Client,
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::min,
    env,
    io::{self, Read, Write},
    path::PathBuf,
    vec,
};
use tokio::{
    fs::{self, OpenOptions},
    io::AsyncWriteExt,
};

/// Program to share files and text.
#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    cmd: Command,
    #[clap(skip)]
    config: Config,
}

/// Represents the CLI subcommands.
#[derive(Clone, Subcommand)]
pub enum Command {
    /// Upload a file to filecrab.
    Upload {
        /// Path to the file to upload.
        #[arg(long, short)]
        path: PathBuf,
        /// Password to protect the file.
        #[arg(long = "password", short = 'P')]
        pwd: Option<String>,
    },
    /// Download the file represented by the ID returned by the upload command.
    Download {
        /// Memorable ID.
        #[arg(long = "file", short = 'f')]
        id: String,
        /// Password to access the file.
        #[arg(long = "password", short = 'P')]
        pwd: Option<String>,
        /// Path to the destination file (default to the current directory).
        #[arg(long, short)]
        path: Option<PathBuf>,
    },
    /// Paste a text and upload it to filecrab.
    Paste {
        /// Text to paste.
        #[arg(long, short)]
        content: String,
        /// Password to protect the text.
        #[arg(long = "password", short = 'P')]
        pwd: String,
    },
    /// Copy the text represented by the ID returned by the paste command to the clipboard.
    Copy {
        /// Memorable ID.
        #[arg(long, short)]
        id: String,
        /// Password to access the text.
        #[arg(long = "password", short = 'P')]
        pwd: String,
    },
}

/// Represents the CLI config.
#[derive(Deserialize, Serialize, Default)]
struct Config {
    url: String,
    api_key: String,
}

/// Represents the response of the upload request.
#[derive(Deserialize)]
struct UploadResponse {
    id: String,
}

/// Represents the body of the paste request.
#[derive(Serialize)]
struct PasteBody {
    content: String,
    #[serde(rename(serialize = "password"))]
    pwd: String,
}

/// Represents the response of the paste request.
#[derive(Deserialize)]
struct PasteResponse {
    id: String,
}

/// Represents the response of the copy request.
#[derive(Deserialize)]
struct CopyResponse {
    content: String,
}

impl Cli {
    /// Runs the CLI.
    pub async fn run(mut self) -> Result<()> {
        // Loads the config.
        self.load_config().await?;

        // Handles the subcommand.
        match self.cmd.clone() {
            Command::Upload { path, pwd } => self.upload(path, pwd).await,
            Command::Download { id, pwd, path } => self.download(id, pwd, path).await,
            Command::Paste { content, pwd } => self.paste(content, pwd).await,
            Command::Copy { id, pwd } => self.copy(id, pwd).await,
        }
    }

    /// Loads the config.
    async fn load_config(&mut self) -> Result<()> {
        // Builds the path to the config file.
        let home_path = match home::home_dir() {
            Some(path) => path,
            None => bail!("Could not locate home path which is mandatory for filecrab"),
        };
        let config_path = home_path.join(".config/filecrab/config.toml");

        // Prompts the user to set the config if it does not exist.
        if !config_path.exists() {
            self.prompt_config(&config_path).await?;
        }

        // Deserializes the config.
        self.config = config::Config::builder()
            .add_source(config::File::from(config_path))
            .build()?
            .try_deserialize()?;
        Ok(())
    }

    /// Prompts the user to set the config and saves it.
    async fn prompt_config(&self, path: &PathBuf) -> Result<()> {
        // Reads the URL from the stdin.
        println!("The config file is not set, we're going to create it.");
        println!("Enter the complete URL of your filecrab (ex: https://my-filecrab-instance.com):");
        let mut url = String::new();
        io::stdin().read_line(&mut url)?;
        let url = url.trim().to_string();

        // Reads the API key from the stdin.
        println!("Enter the API key:");
        let mut api_key = String::new();
        io::stdin().read_line(&mut api_key)?;
        let api_key = api_key.trim().to_string();

        // Builds the config and writes it to the file.
        let parent = match path.parent() {
            Some(parent) => parent,
            None => bail!("Could not retrieve the parent directory of the config file"),
        };
        fs::create_dir_all(parent).await?;
        fs::write(path, &toml::to_string(&Config { url, api_key })?).await?;

        // Prints the completion message.
        println!();
        println!("Thanks, your file has been written in {path:?}. You can modify it manually.");
        println!("Enjoy pinching files and text! BLAZINGLY FAST!");
        println!();
        Ok(())
    }

    /// Uploads a file to filecrab.
    async fn upload(&mut self, path: PathBuf, pwd: Option<String>) -> Result<()> {
        // Destructures the config.
        let Config { url, api_key } = &self.config;

        // Reads the file.
        let mut bytes = fs::read(&path).await?;

        // Initializes the form.
        let mut form = Form::new();

        // If there's a password, adds it to the form and encrypts the file.
        if let Some(pwd) = pwd {
            // Sets the password.
            form = form.text("password", pwd.clone());
            // Encrypts the file.
            bytes = {
                let encryptor = Encryptor::with_user_passphrase(Secret::new(pwd));
                let mut output = Vec::new();
                let mut writer = encryptor.wrap_output(&mut output)?;
                writer.write_all(&bytes)?;
                writer.finish()?;
                output
            };
        }

        // Adds the file to the form.
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|str| str.to_string())
            .unwrap_or_default();
        form = form.part("file", Part::bytes(bytes).file_name(file_name));

        // Sends the request.
        let res: UploadResponse = Client::new()
            .post(format!("{url}/api/upload"))
            .header("filecrab-key", api_key)
            .multipart(form)
            .send()
            .await?
            .json()
            .await?;

        // Prints the ID.
        println!("The ID to share is the following:");
        println!("-> {}", res.id);

        // Copies the ID to the clipboard.
        self.copy_to_clipboard(&res.id)?;
        Ok(())
    }

    /// Downloads a file from filecrab.
    async fn download(
        &mut self,
        id: String,
        pwd: Option<String>,
        path: Option<PathBuf>,
    ) -> Result<()> {
        // Destructures the config.
        let Config { url, api_key } = &self.config;

        // Build the query params.
        let mut query: Vec<(&str, &str)> = vec![("file", &id)];

        // If a password has been set, adds it to the query params.
        if let Some(ref pwd) = pwd {
            query.push(("password", pwd))
        }

        // Sends the request.
        let res = Client::new()
            .get(format!("{url}/api/download"))
            .header("filecrab-key", api_key)
            .query(&query)
            .send()
            .await?;

        // Checks if there's been an error.
        if !res.status().is_success() {
            bail!(format!("{}", res.status().to_string()));
        }

        // Gets the filename from headers.
        let file_name = match res.headers().get("filecrab-file-name") {
            Some(file_name) => file_name.to_str()?.to_string(),
            None => bail!("Could not retrieve the file name from the headers"),
        };

        // Computes the destination path.
        let path = if let Some(path) = path {
            path
        } else {
            env::current_dir()?
        };

        // Creates file with the name of the asset.
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(format!("{}/{}", path.display(), file_name))
            .await?;

        // Gets the content length for the progress bar.
        let total_size = res.content_length().unwrap_or_default();

        // Inits the progress bar.
        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
            .progress_chars("#>-"));
        pb.set_message("Downloading file...");

        // Inits the stream.
        let mut stream = res.bytes_stream();
        let mut downloaded: u64 = 0;

        // Creates the buffer.
        let mut buf: Vec<u8> = Vec::new();

        // Reads the stream.
        while let Some(data) = stream.next().await {
            let chunk = data?;
            tokio::io::copy(&mut chunk.as_ref(), &mut buf).await?;
            let pos = min(downloaded + (chunk.len() as u64), total_size);
            downloaded = pos;
            pb.set_position(pos);
        }

        // Decrypts the file.
        let bytes = if let Some(pwd) = pwd {
            let decryptor = match Decryptor::new(&buf[..])? {
                Decryptor::Passphrase(decryptor) => decryptor,
                _ => unreachable!(),
            };
            let mut output = vec![];
            let mut reader = decryptor.decrypt(&Secret::new(pwd), None)?;
            reader.read_to_end(&mut output)?;
            output
        } else {
            buf
        };

        // Writes the file.
        file.write_all(&bytes).await?;

        // Finishes the progress bar.
        pb.finish_with_message(format!(
            "The name of the downloaded element is: {file_name}"
        ));
        Ok(())
    }

    /// Pastes a text to filecrab.
    async fn paste(&mut self, content: String, pwd: String) -> Result<()> {
        // Destructures the config.
        let Config { url, api_key } = &self.config;

        // Sends the request.
        let res: PasteResponse = Client::new()
            .post(format!("{url}/api/paste"))
            .json(&PasteBody { content, pwd })
            .header("filecrab-key", api_key)
            .send()
            .await?
            .json()
            .await?;

        // Copies the ID to the clipboard.
        self.copy_to_clipboard(&res.id)?;
        Ok(())
    }

    /// Copies a text from filecrab.
    async fn copy(&mut self, id: String, pwd: String) -> Result<()> {
        // Destructures the config.
        let Config { url, api_key } = &self.config;

        // Build the query params.
        let query = vec![("id", id), ("password", pwd)];

        // Sends the request.
        let res: CopyResponse = Client::new()
            .get(format!("{url}/api/copy"))
            .query(&query)
            .header("filecrab-key", api_key)
            .send()
            .await?
            .json()
            .await?;

        // Copies the text to the clipboard.
        self.copy_to_clipboard(&res.content)?;
        Ok(())
    }

    /// Sets the text to the keyboard and waits for the user to CR before returning. This will allow
    /// the user to copy and paste the contents as long as they wish holding the program's exit.
    fn copy_to_clipboard(&self, text: &str) -> Result<()> {
        // Copies the text to the clipboard.
        let mut clipboard = Clipboard::new()?;
        clipboard.set_text(text)?;
        println!("It has now been copied to your clipboard, share it before the program exits!");

        // Prompts the user to press enter to exit.
        println!("Press Enter to exit...");
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        Ok(())
    }
}
