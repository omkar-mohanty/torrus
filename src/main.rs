use clap::Parser;
use clap::Subcommand;
use std::path::PathBuf;
use torrus::metainfo::Metainfo;
use torrus::Client;
use torrus::Result;

#[derive(Parser)]
#[clap(author = "Omkar", version)]
/// A Bittorrent client written in Rust
pub struct Cli {
    /// Command to either download or list all torrents
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Download from a .torrent file
    Download {
        /// Path of the download directory
        #[arg(short, value_name = "DOWNLOAD DIR")]
        output: Option<PathBuf>,
        /// Path to the .torrent file
        path: PathBuf,
    },
    /// List all torrents currently in the client
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Download { output, path } => {
            let file = std::fs::read(path)?;

            let mut metainfo = Metainfo::from_bytes(&file)?;

            metainfo.download_dir = output;

            let mut client = Client::new();

            client.add_torrent_from_metainfo(metainfo)?;

            client.run().await;
        }
        Commands::List => {
            let  client = Client::new();
            client.list_torrents();
        }
    }

    Ok(())
}
