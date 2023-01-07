use clap::ArgGroup;
use metainfo::{render_torrent, Metainfo};
use std::{error::Error, fs, path::PathBuf, time::Duration};
use torrus::{tracker::Peers, *};
use tracker::get_trackers;
use url::Url;

use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    /// Where downloaded files will be stored
    #[arg(short, long)]
    output_dir: PathBuf,
    /// Location of the .torrent file
    #[command(subcommand)]
    command: SubCommand,
    /// Display metainfo file
    #[arg(short, long)]
    display: Option<bool>,
}

#[derive(Subcommand)]

enum SubCommand {
    #[clap(group(
    ArgGroup::new("download")
    .required(true)
    .multiple(true)
    .args(&["magnet","torrent"])
))]
    Download {
        #[clap(short, long)]
        magnet: Option<Url>,
        #[clap(short, long)]
        torrent: Option<PathBuf>,
    },
}

async fn get_peers_from_trackers(path: PathBuf) -> Result<Peers> {
    let buffer = fs::read(path)?;
    let torrent = Metainfo::from_bytes(&buffer)?;

    render_torrent(&torrent);
    println!("Total pieces = {} ", torrent.total_pieces());

    let trackers = get_trackers(&torrent)?;
    let mut addrs = vec![];
    for mut tracker in trackers {
        let time = Duration::from_secs(15);

        let fut = tracker.announce();

        match tokio::time::timeout(time, fut).await {
            Ok(res) => match res {
                Ok(res) => {
                    let peers = res.peers;

                    for peer in peers.addrs {
                        addrs.push(peer);
                    }
                }

                Err(_) => {}
            },
            Err(err) => {
                println!("{}", err);
            }
        }
    }

    Ok(Peers { addrs })
}

type Result<T> = std::result::Result<T, Box<dyn Error>>;
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    env_logger::init();

    let command = cli.command;
    let (path, magnet) = match command {
        SubCommand::Download { magnet, torrent } => (torrent, magnet),
    };

    let mut peers = Vec::new();
    if let Some(path) = path {
        println!("Here");
        let addrs = get_peers_from_trackers(path).await?.addrs;
        peers.push(addrs);
    }

    Ok(())
}
