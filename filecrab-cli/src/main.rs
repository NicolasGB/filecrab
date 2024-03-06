use crate::cli::Cli;
use anyhow::Result;
use clap::Parser;

mod cli;

#[tokio::main]
async fn main() -> Result<()> {
    Cli::parse().run().await
}
