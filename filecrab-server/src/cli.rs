use clap::Parser;

#[derive(Parser)]
pub enum Boot {
    Server,
    Clean,
}
