use futures::{future::join_all, FutureExt, SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::codec::Framed;

use crate::client::TorrentCommand;
use crate::message::Handshake;
use crate::peer::{message_codec::HandShakeCodec, PeerEvent, PeerHandle};
use crate::storage::TorrentFile;
use crate::tracker::Tracker;
use crate::{metainfo::Metainfo, piece::PieceHandler};
use crate::{Hash, Peer, PeerId, Result};
use std::sync::RwLock;

type RwFiles = Vec<RwLock<TorrentFile>>;

pub type PeerEventSender = tokio::sync::mpsc::UnboundedSender<PeerEvent>;
pub type PeerEventReceiver = tokio::sync::mpsc::UnboundedReceiver<PeerEvent>;

pub type TorrentCommandSender = tokio::sync::mpsc::UnboundedSender<TorrentCommand>;
pub type TorrentCommandReceiver = tokio::sync::mpsc::UnboundedReceiver<TorrentCommand>;

pub enum TorrentEvent {
    Command(TorrentCommand),
    PeerEvent(PeerEvent),
}

/// Abstracts over the ways of finding peers in a swarm
struct Discovery {
    /// A optional Vec of trackers in case any present
    trackers: Vec<Tracker>,
    /// 20 Byte PeerID of the client
    peer_id: PeerId,
}

impl Discovery {
    pub fn new(trackers: Vec<Tracker>, peer_id: PeerId) -> Self {
        Self { trackers, peer_id }
    }

    pub async fn get_peers(&mut self) -> Result<Vec<Peer>> {
        let mut peers = Vec::new();

        let responses = join_all(
            self.trackers
                .iter_mut()
                .map(|tracker| tracker.announce(self.peer_id).boxed())
                .collect::<Vec<_>>(),
        )
        .await;

        for response in responses {
            let response = match response {
                Ok(res) => res,
                Err(_) => continue,
            };

            let peer_addrs = &response.peers.addrs;

            peers.extend(peer_addrs);
        }

        Ok(peers)
    }
}

pub struct Context {
    /// Handles all IO operations
    pub piece_handler: RwLock<PieceHandler>,
    /// 20 byte infohash of the torrent
    pub info_hash: Hash,
    /// 20 byte client ID
    pub client_id: PeerId,
    /// V1 Bittorrent metainfo
    pub metainfo: Metainfo,
    /// Channel to send events
    pub sender: PeerEventSender,
}

impl Context {
    pub fn new(
        piece_handler: PieceHandler,
        client_id: PeerId,
        metainfo: Metainfo,
    ) -> Result<(Self, PeerEventReceiver)> {
        let piece_handler = RwLock::new(piece_handler);

        let info_hash = metainfo.hash()?;

        let (sender, receiver) = unbounded_channel();

        Ok((
            Self {
                piece_handler,
                info_hash,
                client_id,
                metainfo,
                sender,
            },
            receiver,
        ))
    }

    pub fn hash(&self) -> &Hash {
        &self.info_hash
    }

    pub fn length(&self) -> u64 {
        self.metainfo.info.length
    }
}

/// High level manager struct which manages the torrent swarm.
pub struct Torrent {
    /// Torrent Metadata
    context: Arc<Context>,
    /// Abstraction for getting peers
    discovery: Discovery,
    /// 20 Byte PeerID of the client
    client_id: PeerId,
    /// hash map of peerID and the coresponding peer handle
    peers: HashMap<PeerId, PeerHandle>,
    /// Channel to receive Torrent events
    peer_event: PeerEventReceiver,
    /// Channel to receive commands from client,
    cmd_rcv: TorrentCommandReceiver,
}

impl Torrent {
    pub fn from_metainfo(
        metainfo: Metainfo,
        client_id: PeerId,
        cmd_rcv: TorrentCommandReceiver,
    ) -> Result<Self> {
        let bitfield = crate::Bitfield::with_capacity(metainfo.total_pieces());

        let files: RwFiles = metainfo
            .get_files()
            .iter()
            .map(|file_info| {
                let torrent_file = TorrentFile::new(file_info.to_owned()).unwrap();

                RwLock::new(torrent_file)
            })
            .collect();

        let piece_handler = PieceHandler::from_metainfo(&metainfo, bitfield, files);

        let (context, receiver) = Context::new(piece_handler, client_id, metainfo)?;

        let context = Arc::new(context);

        let trackers = if let Some(url) = &context.metainfo.announce {
            let tracker = Tracker::from_url_string(url, Arc::clone(&context))?;

            vec![tracker]
        } else if let Some(al) = &context.metainfo.announce_list {
            let mut trackers = Vec::new();

            for a in al {
                let tracker = Tracker::from_url_string(&a[0], Arc::clone(&context))?;
                trackers.push(tracker)
            }

            trackers
        } else {
            vec![]
        };

        let discovery = Discovery::new(trackers, client_id);

        let peers = HashMap::new();

        Ok(Self {
            context,
            discovery,
            client_id,
            peers,
            peer_event: receiver,
            cmd_rcv,
        })
    }

    /// Start Bittorrent `Handshake` protocol with all the peers and then start the wire protocol.
    pub async fn start(&mut self) -> Result<()> {
        let peers = self.discovery.get_peers().await?;

        log::debug!("Got Peers {:?}", peers);

        let results = join_all(
            peers
                .iter()
                .map(|peer| async {
                    let metainfo = Arc::clone(&self.context);

                    let result = connect_to_peer(*peer, metainfo, self.client_id).await?;

                    Ok::<(Handshake, TcpStream), Box<dyn std::error::Error>>(result)
                })
                .collect::<Vec<_>>(),
        )
        .await;

        for result in results {
            let (id, stream) = if let Ok(result) = result {
                let (handshake, stream) = result;

                let id = handshake.peer_id;

                (id, stream)
            } else {
                continue;
            };

            let handle = PeerHandle::new(stream, Arc::clone(&self.context), id);

            self.peers.insert(id, handle);
        }
        self.handle_events().await?;
        Ok(())
    }

    async fn handle_events(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                peer_event = self.peer_event.recv() => {
                    if let Some(event) = peer_event {
                        self.handle_peer_event(event);
                    }

                }

                cmd = self.cmd_rcv.recv() => {

            }
            }
        }
    }

    fn handle_peer_event(&mut self, peer_event: PeerEvent) {
        if let Some(handle) = self.peers.get_mut(&peer_event.peer_id) {
            handle.set_state(peer_event.peer_state);
        }
    }

    fn handle_command(&self, command: TorrentCommand) {
        use TorrentCommand::*;

        match command {
            Progress => {
                unimplemented!("Implement progress tracker");
            }
        }
    }
}

async fn connect_to_peer(
    peer: SocketAddr,
    metainfo: Arc<Context>,
    peer_id: PeerId,
) -> Result<(Handshake, TcpStream)> {
    let stream = TcpStream::connect(peer).await?;

    let mut framed = Framed::new(stream, HandShakeCodec);

    let info_hash = metainfo.hash();
    let handshake = Handshake::new(peer_id, info_hash.to_vec());

    framed.send(handshake).await?;

    loop {
        if let Some(res) = framed.next().await {
            let stream = framed.into_parts().io;
            return Ok((res?, stream));
        } else {
            continue;
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{new_peer_id, Result};

    const FILEPATH: &str = "./resources/ubuntu-22.10-desktop-amd64.iso.torrent";

    #[tokio::test]
    async fn create_from_metainfo() -> Result<()> {
        let file = std::fs::read(FILEPATH)?;
        let metainfo = Metainfo::from_bytes(&file).unwrap();
        let peer_id = new_peer_id();
        let (_, receiver) = unbounded_channel();
        let _ = Torrent::from_metainfo(metainfo, peer_id, receiver);

        Ok(())
    }
}
