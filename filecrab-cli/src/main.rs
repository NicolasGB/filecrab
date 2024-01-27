use clap::Parser;

use crate::cli::Cli;

mod cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse the cli
    let app = Cli::parse();
    // Run the app
    app.run().await
}
