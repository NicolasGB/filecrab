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
    io::{self, IsTerminal, Read, Write},
    path::PathBuf,
    time::Duration,
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
        path: PathBuf,
        /// Password to protect the file.
        #[arg(long, short = 's')]
        pwd: Option<String>,
    },
    /// Download the file represented by the ID returned by the upload command.
    Download {
        /// Memorable ID.
        id: String,
        /// Password to access the file.
        #[arg(long, short = 's')]
        pwd: Option<String>,
        /// Path to the destination file (default to the current directory).
        #[arg(long, short)]
        path: Option<PathBuf>,
    },
    /// Paste a text and upload it to filecrab. Content can be either specified positionally or
    /// piped.
    Paste {
        /// Text to paste.
        content: Option<String>,
        /// Password to protect the text.
        #[arg(long, short = 's')]
        pwd: String,
    },
    /// Copy the text represented by the ID returned by the paste command to the clipboard.
    Copy {
        /// Memorable ID.
        id: String,
        /// Password to access the text.
        pwd: String,
        /// Optional OUT file to write the contents to (ex. myfile.txt).
        #[arg(long, short)]
        out: Option<PathBuf>,
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
            Command::Paste { content, pwd } => match content {
                Some(content) => self.paste(content, pwd).await,
                None if io::stdin().is_terminal() => bail!("You have not provided specific text nor piped anything. Run `filecrab paste -h` to understand the command."),
                None => {
                        let mut content = String::new();
                        io::stdin().lock().read_to_string(&mut content)?;
                        self.paste(content.trim().to_string(), pwd).await
                }
            },
            Command::Copy { id, pwd, out } => self.copy(id, pwd, out).await,
        }
    }

    /// Loads the config.
    async fn load_config(&mut self) -> Result<()> {
        // Builds the path to the config file.
        let config_path = match dirs::config_dir() {
            Some(config_dir) => config_dir.join("filecrab/config.toml"),
            None => bail!("Could not locate config directory which is mandatory for filecrab"),
        };

        // Prompts the user to set the config if it does not exist.
        if !config_path.exists() {
            self.prompt_config(&config_path).await?;
        }

        // Deserializes the config.
        self.config = toml::from_str(&fs::read_to_string(&config_path).await?)?;
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
            form = form.text("encrypted", "true");
            // Encrypts the file.
            let mut bar = ProgressBar::new_spinner();
            bar = bar.with_message("Encrypting file");
            bar.enable_steady_tick(Duration::from_millis(100));

            bytes = Cli::encrypt_slice(&bytes, pwd)?;
            bar.finish_with_message("File encrypted.")
        }

        // Adds the file to the form.
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|str| str.to_string())
            .unwrap_or_default();
        form = form.part("file", Part::bytes(bytes).file_name(file_name));

        // Set Upload bar
        let mut bar = ProgressBar::new_spinner();
        bar = bar.with_message("Uploading to filecrab. ");
        bar.enable_steady_tick(Duration::from_millis(100));

        // Sends the request.
        let res: UploadResponse = Client::new()
            .post(format!("{url}/api/upload"))
            .header("filecrab-key", api_key)
            .multipart(form)
            .send()
            .await?
            .json()
            .await?;
        bar.finish_with_message("File correctly uploaded.");

        // Prints the ID.
        println!("The ID to share is the following:");
        println!("-> {}", res.id);
        println!();

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
        let query: Vec<(&str, &str)> = vec![("file", &id)];

        // Set Upload bar
        let mut bar = ProgressBar::new_spinner();
        bar = bar.with_message("Requesting file to filecrab.");
        bar.enable_steady_tick(Duration::from_millis(100));

        // Sends the request.
        let res = Client::new()
            .get(format!("{url}/api/download"))
            .header("filecrab-key", api_key)
            .query(&query)
            .send()
            .await?;
        bar.finish();

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
            let mut bar = ProgressBar::new_spinner();
            bar = bar.with_message("Decrypting file");
            bar.enable_steady_tick(Duration::from_millis(100));

            let output = Cli::decrypt_slice(&buf[..], pwd)?;
            bar.finish();
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

        // Set the spinner
        let mut bar = ProgressBar::new_spinner();
        bar = bar.with_message("Encrypting text");
        bar.enable_steady_tick(Duration::from_millis(100));

        // Encrypt the text
        let encrypted_bytes = Cli::encrypt_slice(content.as_bytes(), pwd)?;
        let content = hex::encode(encrypted_bytes);
        bar.finish_with_message("Text successfully encrypted.");

        // Sends the request.
        let res = Client::new()
            .post(format!("{url}/api/paste"))
            .json(&PasteBody { content })
            .header("filecrab-key", api_key)
            .send()
            .await?;

        // Checks if there's been an error.
        if !res.status().is_success() {
            bail!(format!("{}", res.status().to_string()));
        }

        let body: PasteResponse = res.json().await?;

        println!("The ID to share is the following:");
        println!("-> {}", body.id);
        println!();
        // Copies the ID to the clipboard.
        self.copy_to_clipboard(&body.id)?;
        Ok(())
    }

    /// Copies a text from filecrab to the user's clipboard or, if set, to a given file.
    async fn copy(&mut self, id: String, pwd: String, out: Option<PathBuf>) -> Result<()> {
        //Check if a file has been given, if so check it's falid
        let file = if let Some(path) = out {
            // Creates file with the name of the asset.
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(format!("{}", path.display()))
                .await?;
            Some(file)
        } else {
            None
        };

        // Destructures the config.
        let Config { url, api_key } = &self.config;

        // Build the query params.
        let query = vec![("memo_id", id)];

        // Sends the request.
        let res = Client::new()
            .get(format!("{url}/api/copy"))
            .query(&query)
            .header("filecrab-key", api_key)
            .send()
            .await?;

        // Checks if there's been an error.
        if !res.status().is_success() {
            bail!(format!("{}", res.status().to_string()));
        }

        let body: CopyResponse = res.json().await?;

        // Set the spinner
        let mut bar = ProgressBar::new_spinner();
        bar = bar.with_message("Decrypting text");
        bar.enable_steady_tick(Duration::from_millis(100));

        // Decrypt the text
        let encrypted_bytes = hex::decode(body.content.as_bytes())?;
        let content = Cli::decrypt_slice(&encrypted_bytes[..], pwd)?;
        let content = String::from_utf8_lossy(&content);
        bar.finish_and_clear();

        if let Some(mut file) = file {
            file.write_all(content.as_bytes()).await?;
        } else {
            // Copies the text to the clipboard.
            self.copy_to_clipboard(&content)?;
        }

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

    /// Given a slice of bytes and a password, tries to decrypt it's values and returns the
    /// original content.
    /// Uses the age algorithm.
    fn decrypt_slice(buf: &[u8], pwd: String) -> Result<Vec<u8>, anyhow::Error> {
        let decryptor = match Decryptor::new(buf)? {
            Decryptor::Passphrase(decryptor) => decryptor,
            _ => unreachable!(),
        };
        let mut output = vec![];
        let mut reader = decryptor.decrypt(&Secret::new(pwd), None)?;
        reader.read_to_end(&mut output)?;
        Ok(output)
    }

    /// Given a slice of bytes and a password encrypts the value and returns the resulting encryption.
    fn encrypt_slice(bytes: &[u8], pwd: String) -> Result<Vec<u8>, anyhow::Error> {
        let encryptor = Encryptor::with_user_passphrase(Secret::new(pwd));
        let mut output = Vec::new();
        let mut writer = encryptor.wrap_output(&mut output)?;
        writer.write_all(bytes)?;
        writer.finish()?;
        Ok(output)
    }
}
