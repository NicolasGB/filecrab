use std::{io, string::FromUtf8Error};

use age::{DecryptError, EncryptError};
use indicatif::style::TemplateError;
use inquire::InquireError;
use reqwest::header::ToStrError;
use thiserror::Error;

pub type Result<T = ()> = core::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    // Config
    #[error("Could not locate config directory which is mandatory for filecrab.")]
    ConfigNotFound,
    #[error("could not create filecrab config directory")]
    CreateConfigDir(#[source] io::Error),
    #[error("could not retrieve the parent directory of the config file")]
    NoParentDir,
    #[error("there are no other instances of filecrab currently")]
    NoOtherInstances,
    #[error("the instance you are trying to set does not exist within filecrab's config")]
    InstanceNotFound,
    #[error("there can not be two instances with the same name {0}")]
    DuplicateInstanceName(String),
    #[error("Filecrab has already been initialized, a config already exists.")]
    ConfigExists,
    #[error("Error trying to find filecrab's config")]
    FindConfig(#[source] io::Error),

    // Toml
    #[error("could not parse config toml: {0}")]
    ParseToml(#[source] toml::de::Error),
    #[error("could not serialize config toml")]
    SerializeToml(#[source] toml::ser::Error),

    // Std in
    #[error("could not lock std in")]
    LockStdIn(#[source] io::Error),
    #[error("could not read std in")]
    ReadStdIn(#[source] io::Error),

    // Pipe
    #[error("You have not provided specific text nor piped anything. Run `filecrab paste -h` to understand the command.")]
    NoPipedContent,

    // Files and Dirs
    #[error("could not read from file {path}")]
    ReadFile { path: String, source: io::Error },
    #[error("could not write to file {path}")]
    WriteFile { path: String, source: io::Error },
    #[error("could not get current dir")]
    CurrentDir(#[source] io::Error),
    #[error("could not open file: {path}, please make sure the file doesn't already exist")]
    OpenFile { path: String, source: io::Error },
    #[error("could not delete temporary out file")]
    DeleteTempFile,
    #[error("could not delete config file")]
    RemoveConfig(#[source] io::Error),

    // Tokio
    #[error("could not copy received chunk")]
    CopyChunk(#[source] io::Error),

    // Encryption
    #[error("failed to wrap writer with encryptor")]
    EncryptionWriterWrap(#[source] EncryptError),
    #[error("failed to finish encryption")]
    FinishEncryption(#[source] io::Error),
    #[error("failed to create decryptor")]
    CreateDecryptor(#[source] DecryptError),
    #[error("failed to decrypt data")]
    FailedToDecrypt(#[source] DecryptError),

    // Reader and Writer
    #[error("could not write to {r#type} writer")]
    WriteToWriter { r#type: String, source: io::Error },
    #[error("could not read from {r#type} reader")]
    ReadFromReader { r#type: String, source: io::Error },

    //Http
    #[error("reqwest error")]
    Reqwest(#[from] reqwest::Error),
    #[error("could not parse json from reqwest response")]
    ReqwestJsonParse(#[source] reqwest::Error),
    #[error("could not read reqwest body")]
    ReqwestReadBody(#[source] reqwest::Error),

    // Filecrab Response
    #[error("Unsuccessful request. \nStatus: {status}\nBody: {body}")]
    UnsuccessfulRequest { status: String, body: String },
    #[error("could not retrieve the file name from the headers")]
    MissingFileNameInHeaders,

    // String
    #[error("could not parse utf8 bytes")]
    Utf8Parse(#[from] FromUtf8Error),
    #[error("could not convert header to string slice")]
    ToStr(#[from] ToStrError),

    // Clipboard
    #[error("could not read from clipboard")]
    ReadFromClipboard(#[from] arboard::Error),

    // Hex
    #[error("could not decode hex")]
    DecodeHex(#[from] hex::FromHexError),

    // Progressbar
    #[error(transparent)]
    Template(#[from] TemplateError),

    // Inquire
    #[error("could not prompt the user")]
    Inquire(#[from] InquireError),
    #[error("Canceled.")]
    UserCancel,
}
