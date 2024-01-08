use std::path::PathBuf;

use clap::Parser;
use torrus::client::{default_client, Client, ClientConfig};
use torrus::error::Result;

#[derive(Parser)]
pub struct Cli {
    torrents: Vec<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let client = default_client();

    log::info!("Adding torrents");

    client.run().await.unwrap();

    for torrent in cli.torrents {
        println!("Here");
        client.add_torrent(torrent).await?;
    }

    log::info!("Initializing client");
    println!("Hello, world!");
    Ok(())
}
