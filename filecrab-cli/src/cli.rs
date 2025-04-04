mod config;

use crate::{Result, cli::config::Instance, error::Error};
use age::{Decryptor, Encryptor, secrecy::SecretString};
use anstyle::AnsiColor;
use arboard::Clipboard;
use clap::{Parser, Subcommand, builder::Styles};
use config::Config;
use file_format::FileFormat;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::Confirm;
use reqwest::{
    Client,
    multipart::{Form, Part},
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::min,
    env,
    io::{self, IsTerminal, Read, Write},
    iter,
    path::{Path, PathBuf},
    time::Duration,
    vec,
};
use tokio::{
    fs::{self, OpenOptions},
    io::AsyncWriteExt,
};

const COPY_COMMAND: &str = "filecrab copy";
const DOWNLOAD_COMMAND: &str = "filecrab download";

/// Program to share files and text.
#[derive(Parser)]
#[command(styles=Self::styles())]
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
        #[arg(long)]
        pwd: Option<String>,
    },
    /// Download the file represented by the ID returned by the upload command.
    Download {
        /// Memorable ID.
        id: String,
        /// Password to access the file.
        #[arg(long)]
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
        #[arg(long)]
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
    /// Switches the active instance in filecrab.
    Switch,
    /// Adds a new filecrab instance to the config.
    Add,
    /// Removes a filecrab instance from the config.
    Remove,
    /// Inits the config for filecrab
    Init,
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

// Implementation of the commands of Cli.
impl Cli {
    /// Returns the styles for the CLI.
    fn styles() -> clap::builder::Styles {
        Styles::styled()
            .header(AnsiColor::Yellow.on_default())
            .usage(AnsiColor::Yellow.on_default())
            .literal(AnsiColor::Green.on_default())
            .placeholder(AnsiColor::Blue.on_default())
    }

    /// Runs the CLI.
    pub async fn run(mut self) -> Result {
        // Check if the command is an init
        if let Command::Init = self.cmd {
            return self.init().await;
        }

        // Loads the config.
        self.config = Config::load_config().await?;

        // Handles the subcommand.
        match self.cmd.clone() {
            Command::Upload { path, pwd } => self.upload(path, pwd).await,
            Command::Download { id, pwd, path } => self.download(id, pwd, path).await,
            Command::Paste { content, pwd } => match content {
                Some(content) => self.paste(content, pwd).await,
                None if io::stdin().is_terminal() => Err(Error::NoPipedContent),
                None => {
                    let mut content = String::new();
                    io::stdin()
                        .lock()
                        .read_to_string(&mut content)
                        .map_err(Error::LockStdIn)?;
                    self.paste(content.trim().to_string(), pwd).await
                }
            },
            Command::Copy { id, pwd, out } => self.copy(id, pwd, out).await,
            Command::Switch => self.switch().await,
            Command::Add => self.add().await,
            Command::Remove => self.remove().await,
            _ => unreachable!(),
        }
    }

    /// Uploads a file to filecrab.
    async fn upload(&mut self, path: PathBuf, mut pwd: Option<String>) -> Result<()> {
        // Destructures the config.
        let Instance { url, api_key, name } = &self.config.get_active_instance();
        println!("Active filecrab instance: {name}");

        // Reads the file.
        let mut bytes = fs::read(&path).await.map_err(|err| Error::ReadFile {
            path: format!("{}", path.display()),
            source: err,
        })?;

        // Initializes the form.
        let mut form = Form::new();

        // Prompt the user for a password
        if pwd.is_none()
            && Confirm::new("Do you wish to encrypt the file?")
                .with_default(false)
                .prompt()?
        {
            let given_pwd = inquire::prompt_text("Password to use for encryption:")?;
            pwd = Some(given_pwd);
        };

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
        let res = Client::new()
            .post(format!("{url}/api/upload"))
            .header("filecrab-key", api_key)
            .multipart(form)
            .send()
            .await?;

        // Checks if there's been an error.
        if !res.status().is_success() {
            let status = res.status().to_string();
            let body = res.bytes().await.map_err(Error::ReqwestReadBody)?;
            let body = String::from_utf8(body.to_vec())?;
            return Err(Error::UnsuccessfulRequest { status, body });
        }

        let res: UploadResponse = res.json().await.map_err(Error::ReqwestJsonParse)?;
        bar.finish_with_message("File correctly uploaded.");

        // Prints the ID.
        println!("The ID to share is the following:");
        println!("-> {}", res.id);
        println!();

        // Copies the ID to the clipboard.
        self.copy_to_clipboard(Some(DOWNLOAD_COMMAND), &res.id)?;
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
        let Instance { url, api_key, name } = &self.config.get_active_instance();
        println!("Active filecrab instance: {name}");

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
            let status = res.status().to_string();
            let body = res.bytes().await.map_err(Error::ReqwestReadBody)?;
            let body = String::from_utf8(body.to_vec())?;
            return Err(Error::UnsuccessfulRequest { status, body });
        }

        // Gets the filename from headers.
        let file_name = match res.headers().get("filecrab-file-name") {
            Some(file_name) => file_name.to_str()?.to_string(),
            None => return Err(Error::MissingFileNameInHeaders),
        };

        // Computes the destination path.
        let path = if let Some(path) = path {
            path
        } else {
            env::current_dir().map_err(Error::CurrentDir)?
        };

        // Gets the content length for the progress bar.
        let total_size = res.content_length().unwrap_or_default();

        // Inits the progress bar.
        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
            .progress_chars("█░"));
        pb.set_message("Downloading file...");

        // Inits the stream.
        let mut stream = res.bytes_stream();
        let mut downloaded: u64 = 0;

        // Creates the buffer.
        let mut buf: Vec<u8> = Vec::new();

        // Reads the stream.
        while let Some(data) = stream.next().await {
            let chunk = data?;
            tokio::io::copy(&mut chunk.as_ref(), &mut buf)
                .await
                .map_err(Error::CopyChunk)?;
            let pos = min(downloaded + (chunk.len() as u64), total_size);
            downloaded = pos;
            pb.set_position(pos);
        }
        // Finishes the progress bar.
        pb.finish();

        // Decrypts the file.
        let bytes = if let Some(pwd) = pwd {
            let mut bar = ProgressBar::new_spinner();
            bar = bar.with_message("Decrypting file");
            bar.enable_steady_tick(Duration::from_millis(100));

            let output = Cli::decrypt_slice(&buf[..], pwd)?;
            bar.finish();
            output
        } else if FileFormat::from_bytes(&buf) == FileFormat::AgeEncryption {
            // If the data coming in is encrypted, Prompt the user for a password
            let pwd = inquire::prompt_text("The file is encrypted, please provide a password:")?;

            let mut bar = ProgressBar::new_spinner();
            bar = bar.with_message("Decrypting file");
            bar.enable_steady_tick(Duration::from_millis(100));

            let output = Cli::decrypt_slice(&buf[..], pwd)?;
            bar.finish();
            output
        } else {
            buf
        };

        // Creates file with the name of the asset.
        let mut file = Cli::create_file(format!("{}/{}", path.display(), file_name)).await?;

        // Writes the file.
        file.write_all(&bytes)
            .await
            .map_err(|err| Error::WriteFile {
                path: format!("{}", path.display()),
                source: err,
            })?;

        println!("The name of the downloaded element is: {file_name}");
        Ok(())
    }

    /// Pastes a text to filecrab.
    async fn paste(&mut self, content: String, pwd: String) -> Result<()> {
        // Destructures the config.
        let Instance { url, api_key, name } = &self.config.get_active_instance();
        println!("Active filecrab instance: {name}");

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
            let status = res.status().to_string();
            let body = res.bytes().await.map_err(Error::ReqwestReadBody)?;
            let body = String::from_utf8(body.to_vec())?;
            return Err(Error::UnsuccessfulRequest { status, body });
        }

        let body: PasteResponse = res.json().await?;

        println!("The ID to share is the following:");
        println!("-> {}", body.id);
        println!();
        // Copies the command to retrieve the text and the ID to the clipboard.
        self.copy_to_clipboard(Some(COPY_COMMAND), &body.id)?;
        Ok(())
    }

    /// Copies a text from filecrab to the user's clipboard or, if set, to a given file.
    async fn copy(&mut self, id: String, pwd: String, out: Option<PathBuf>) -> Result<()> {
        //Check if a file has been given, if so check it's falid
        if let Some(ref path) = out {
            Cli::check_file_can_be_created(path).await?;
        }

        // Destructures the config.
        let Instance {
            url,
            api_key,
            name: _,
        } = &self.config.get_active_instance();

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
            let status = res.status().to_string();
            let body = res.bytes().await.map_err(Error::ReqwestReadBody)?;
            let body = String::from_utf8(body.to_vec())?;
            return Err(Error::UnsuccessfulRequest { status, body });
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

        if let Some(path) = out {
            let mut file = Cli::create_file(path).await?;

            file.write_all(content.as_bytes())
                .await
                .map_err(|err| Error::WriteToWriter {
                    r#type: String::from("file"),
                    source: err,
                })?;
        } else {
            // Copies the text to the clipboard.
            self.copy_to_clipboard(None, &content)?;
        }

        Ok(())
    }

    // Switches the filecrab instance
    async fn switch(&mut self) -> Result {
        self.config.switch_instance().await
    }

    /// Allows the user to add a new filecrab instance to the config.
    async fn add(&mut self) -> Result {
        self.config.add().await
    }

    /// Allows the user to remove a filecrab instance from the config.
    async fn remove(&mut self) -> Result {
        self.config.remove().await
    }

    /// Allows the user to initialize a filecrab config.
    async fn init(&mut self) -> Result {
        self.config.init().await
    }
}

// Implementation of the helper functions in Cli.
impl Cli {
    /// Sets the text to the keyboard and waits for the user to CR before returning. This will allow
    /// the user to copy and paste the contents as long as they wish holding the program's exit.
    fn copy_to_clipboard(&self, command: Option<&str>, text: &str) -> Result<()> {
        let mut clipboard = Clipboard::new()?;
        // Copies the command and the text to the clipboard.
        if let Some(command) = command {
            clipboard.set_text(format!("{command} {text}"))?;
            println!(
                "The resulting command has now been copied to your clipboard, share it before the program exits!"
            );
        } else {
            // Copies the text to the clipboard.
            clipboard.set_text(text)?;
            println!(
                "The text has now been copied to your clipboard, share it before the program exits!"
            );
        }

        // Prompts the user to press enter to exit.
        println!("Press Enter to exit...");
        let mut buf = String::new();
        io::stdin().read_line(&mut buf).map_err(Error::ReadStdIn)?;
        Ok(())
    }

    /// Given a slice of bytes and a password, tries to decrypt it's values and returns the
    /// original content.
    /// Uses the age algorithm.
    fn decrypt_slice(buf: &[u8], pwd: String) -> Result<Vec<u8>> {
        let decryptor = Decryptor::new(buf).map_err(Error::CreateDecryptor)?;

        let mut output = vec![];
        let mut reader = decryptor
            .decrypt(iter::once(
                &age::scrypt::Identity::new(SecretString::from(pwd)) as _,
            ))
            .map_err(Error::FailedToDecrypt)?;
        reader
            .read_to_end(&mut output)
            .map_err(|err| Error::ReadFromReader {
                r#type: String::from("decrypt"),
                source: err,
            })?;
        Ok(output)
    }

    /// Given a slice of bytes and a password encrypts the value and returns the resulting encryption.
    fn encrypt_slice(bytes: &[u8], pwd: String) -> Result<Vec<u8>> {
        let encryptor = Encryptor::with_user_passphrase(SecretString::from(pwd));
        let mut output = Vec::new();
        let mut writer = encryptor
            .wrap_output(&mut output)
            .map_err(Error::EncryptionWriterWrap)?;
        writer
            .write_all(bytes)
            .map_err(|err| Error::WriteToWriter {
                r#type: String::from("encryption"),
                source: err,
            })?;
        writer.finish().map_err(Error::FinishEncryption)?;
        Ok(output)
    }

    /// Checks if a file can be created, removes the created file right after
    async fn check_file_can_be_created(path: impl AsRef<Path>) -> Result {
        println!("{:?}", path.as_ref());
        let _ = Cli::create_file(&path).await?;

        // Now that we know the file can be oppened and created when delete it.
        fs::remove_file(path)
            .await
            .map_err(|_| Error::DeleteTempFile)
    }

    /// Creates a file given a path
    async fn create_file(path: impl AsRef<Path>) -> Result<fs::File> {
        // Creates file with the name of the asset.
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
            .map_err(|err| Error::OpenFile {
                path: format!("{}", path.as_ref().display()),
                source: err,
            })
    }
}
