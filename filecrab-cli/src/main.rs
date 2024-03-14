use crate::cli::Cli;
use clap::Parser;

mod cli;
mod error;

pub use error::Result;

#[tokio::main]
async fn main() {
    Cli::parse()
        .run()
        .await
        .unwrap_or_else(|err| eprintln!("Error: {err}"))
}
