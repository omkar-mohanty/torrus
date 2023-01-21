use clap::Parser;
use std::path::PathBuf;
use torrus::metainfo::Metainfo;
use torrus::Client;
use torrus::Result;

#[derive(Parser)]
pub struct Cli {
    /// Path to the .torrent file
    path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let file = std::fs::read(cli.path)?;

    let metainfo = Metainfo::from_bytes(&file)?;

    let mut client = Client::new();

    client.add_torrent_from_metainfo(metainfo)?;
    Ok(())
}
