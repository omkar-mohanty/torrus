use clap::Parser;
use std::fmt::Display;
use std::path::PathBuf;
use torrus::metainfo::Metainfo;
use torrus::Client;
use torrus::Result;

#[derive(Parser)]
pub struct Cli {
    /// Path of the download directory
    #[arg(short, value_name="DOWNLOAD DIR")]
    output: Option<PathBuf>,
    /// Path to the .torrent file
    path: PathBuf,
}

impl Display for Cli {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
       f.write_str(&format!("Download directory:\t{:?}", self.output)) 
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    log::info!("{}", cli);

    let file = std::fs::read(cli.path)?;

    let mut metainfo = Metainfo::from_bytes(&file)?;
    
    metainfo.download_dir = cli.output;
    
    let mut client = Client::new();

    client.add_torrent_from_metainfo(metainfo)?;

    client.run().await;
    Ok(())
}
