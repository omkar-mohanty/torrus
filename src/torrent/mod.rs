use futures::{future::join_all, FutureExt, SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, RwLock};
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tokio_util::codec::Framed;

use crate::block::Block;
use crate::client::TorrentCommand;
use crate::error::TorrusError;
use crate::message::Handshake;
use crate::peer::{message_codec::HandShakeCodec, new_peer, Peer};
use crate::storage::TorrentFile;
use crate::tracker::Tracker;
use crate::{metainfo::Metainfo, piece::PieceHandler};
use crate::{Hash, PeerAddr, PeerId, Result};

type RwFiles = Vec<RwLock<TorrentFile>>;

pub type TorrentCommandSender = tokio::sync::mpsc::UnboundedSender<TorrentCommand>;
pub type TorrentCommandReceiver = tokio::sync::mpsc::UnboundedReceiver<TorrentCommand>;

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

    pub async fn get_peers(&mut self, num_want: i32, port: u16) -> Result<Vec<PeerAddr>> {
        let mut peers = Vec::new();

        let responses = join_all(
            self.trackers
                .iter_mut()
                .map(|tracker| tracker.announce(self.peer_id, num_want, port).boxed())
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
    pub piece_handler: Mutex<PieceHandler>,
    /// 20 byte infohash of the torrent
    pub info_hash: Hash,
    /// 20 byte client ID
    pub client_id: PeerId,
    /// V1 Bittorrent metainfo
    pub metainfo: Metainfo,
}

impl Context {
    pub fn new(piece_handler: PieceHandler, client_id: PeerId, metainfo: Metainfo) -> Result<Self> {
        let piece_handler = Mutex::new(piece_handler);

        let info_hash = metainfo.hash()?;

        Ok(Self {
            piece_handler,
            info_hash,
            client_id,
            metainfo,
        })
    }

    pub fn hash(&self) -> &Hash {
        &self.info_hash
    }

    pub fn length(&self) -> u64 {
        self.metainfo.info.length
    }

    fn get_mutex<F, T>(&self, func: F) -> Result<T>
    where
        F: FnOnce(MutexGuard<PieceHandler>) -> T,
    {
        let handler = self.piece_handler.lock().unwrap();

        // Critical section
        let ret = func(handler);

        Ok(ret)
    }

    pub fn match_bitfield_len(&self, len: usize) -> Result<bool> {
        self.get_mutex(|handler| handler.match_bitfield_len(len))
    }

    pub fn insert_block(&self, block: Block) -> Result<()> {
        self.get_mutex(|mut handler| handler.insert_block(block))?
    }
}

/// High level manager struct which manages the torrent swarm.
pub struct Torrent {
    /// Torrent Metadata
    context: Arc<Context>,
    /// 20 Byte PeerID of the client
    client_id: PeerId,
    /// hash map of peerID and the coresponding peer handle
    peers: HashMap<PeerId, Peer>,
    /// Peer Discovery
    discovery: Discovery,
}

impl Torrent {
    pub fn from_metainfo(metainfo: Metainfo, client_id: PeerId) -> Result<Self> {
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

        let context = Context::new(piece_handler, client_id, metainfo)?;

        let context = Arc::new(context);

        let peers = HashMap::new();

        let trackers = Self::get_trackers(&context)?;

        let discovery = Discovery::new(trackers, client_id);

        Ok(Self {
            context,
            client_id,
            peers,
            discovery,
        })
    }

    async fn insert_new_peer_stream(&mut self, stream: TcpStream) -> Result<()> {
        let stream = Framed::new(stream, HandShakeCodec);

        let fut = handshake_timeout(stream);

        match timeout(Duration::from_secs(10), fut).await {
            Ok(res) => {
                let (handshake, stream) = res?;

                let id = handshake.peer_id;

                let handle = new_peer(stream, Arc::clone(&self.context), id);

                self.peers.insert(id, handle);

                Ok(())
            }
            Err(_) => Err(TorrusError::new(
                "Remote Peer could not send handshake in time",
            )),
        }
    }

    async fn get_peers(&mut self, port: u16) -> Option<Vec<PeerAddr>> {
        if self.peers.len() < 30 {
            let peers = self
                .discovery
                .get_peers(30 - self.peers.len() as i32, port)
                .await;

            match peers {
                Ok(peers) => Some(peers),
                Err(_) => None,
            }
        } else {
            None
        }
    }

    async fn get_peer_streams(&mut self, peers: Vec<PeerAddr>) {
        log::debug!("\tget_peer_streams : Got Peers {}", peers.len());

        let results = join_all(
            peers
                .iter()
                .map(|peer| async {
                    let metainfo = Arc::clone(&self.context);

                    let result = connect_to_peer(*peer, metainfo, self.client_id).await?;

                    Ok::<(Handshake, TcpStream), TorrusError>(result)
                })
                .collect::<Vec<_>>(),
        )
        .await;

        for result in results {
            let (id, stream) = match result {
                Ok((handshake, stream)) => {
                    let id = handshake.peer_id;

                    (id, stream)
                }
                Err(err) => {
                    log::error!("\tstart : Error:\t{}", err);
                    continue;
                }
            };

            log::debug!("\tstart : Handshake successful with Peer");

            let handle = new_peer(stream, Arc::clone(&self.context), id);

            self.peers.insert(id, handle);
        }
    }

    fn get_trackers(context: &Arc<Context>) -> Result<Vec<Tracker>> {
        let trackers = if let Some(url) = &context.metainfo.announce {
            let tracker = Tracker::from_url_string(&url, Arc::clone(context))?;

            vec![tracker]
        } else if let Some(al) = &context.metainfo.announce_list {
            let mut trackers = Vec::new();

            for a in al {
                let tracker = Tracker::from_url_string(&a[0], Arc::clone(context))?;
                trackers.push(tracker)
            }

            trackers
        } else {
            vec![]
        };

        Ok(trackers)
    }

    /// Start Bittorrent `Handshake` protocol with all the peers and then start the wire protocol.
    pub async fn start(&mut self, rx: TorrentCommandReceiver) -> Result<()> {
        self.handle_events(rx).await?;

        Ok(())
    }

    async fn handle_events(&mut self, mut rx: TorrentCommandReceiver) -> Result<()> {
        let addr: SocketAddr = "0.0.0.0:0".parse().unwrap();

        let port = addr.port();

        let listner = TcpListener::bind(addr).await?;

        loop {
            tokio::select! {
             Some(cmd) = rx.recv() => {
                self.handle_command(cmd);
             }
             res = listner.accept() => {
                if let Ok((stream, _)) = res {
                       let _ =  self.insert_new_peer_stream(stream);
                    }
                }
             Some(peers) = self.get_peers(port) => {
                     self.get_peer_streams(peers).await;
                }
            }
        }
    }

    fn handle_command(&self, command: TorrentCommand) {
        use TorrentCommand::*;

        match command {
            Progress => {
                todo!("Implement Progress");
            }
        }
    }
}

async fn connect_to_peer(
    peer: SocketAddr,
    metainfo: Arc<Context>,
    peer_id: PeerId,
) -> Result<(Handshake, TcpStream)> {
    log::debug!("\tconnect_to_peer : connect to Peer");

    let fut = TcpStream::connect(peer);

    let timeout_dur = Duration::from_secs(10);

    let stream = match timeout(timeout_dur, fut).await {
        Ok(stream) => stream?,
        Err(_) => {
            return Err(TorrusError::new(
                "Could not connect to Peer within 10 seconds",
            ))
        }
    };

    log::debug!("\tconnect_to_peer : successful");

    let mut framed = Framed::new(stream, HandShakeCodec);

    let info_hash = metainfo.hash();
    let handshake = Handshake::new(peer_id, info_hash.to_vec());

    let fut = framed.send(handshake);

    if timeout(timeout_dur, fut).await.is_err() {
        return Err(TorrusError::new("Could not send handshake in 10 seconds"));
    }

    let fut = handshake_timeout(framed);

    match timeout(timeout_dur, fut).await {
        Ok(res) => Ok(res?),
        Err(_) => Err(TorrusError::new(
            "Could not get Handshake within 10 seconds",
        )),
    }
}

async fn handshake_timeout(
    mut framed: Framed<TcpStream, HandShakeCodec>,
) -> Result<(Handshake, TcpStream)> {
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
    use crate::{new_peer_id, Bitfield, Result};

    const FILEPATH: &str = "./resources/ubuntu-22.10-desktop-amd64.iso.torrent";

    #[tokio::test]
    async fn create_from_metainfo() -> Result<()> {
        let file = std::fs::read(FILEPATH)?;
        let metainfo = Metainfo::from_bytes(&file).unwrap();
        let peer_id = new_peer_id();
        let _ = Torrent::from_metainfo(metainfo, peer_id);
        Ok(())
    }

    #[test]
    fn test_context() -> Result<()> {
        let file = std::fs::read(FILEPATH)?;
        let metainfo = Metainfo::from_bytes(&file).unwrap();
        let piece_handler = PieceHandler::from_metainfo(&metainfo, Bitfield::new(), vec![]);
        let client_id = new_peer_id();
        let context = Context::new(piece_handler, client_id, metainfo)?;

        let val = context.match_bitfield_len(Bitfield::new().len())?;

        assert!(val);
        Ok(())
    }

    #[tokio::test]
    async fn test_context_threaded() -> Result<()> {
        let file = std::fs::read(FILEPATH)?;
        let metainfo = Metainfo::from_bytes(&file).unwrap();
        let piece_handler = PieceHandler::from_metainfo(&metainfo, Bitfield::new(), vec![]);
        let client_id = new_peer_id();
        let context = Arc::new(Context::new(piece_handler, client_id, metainfo)?);

        let context1 = Arc::clone(&context);

        let context2 = Arc::clone(&context);

        let handle1 = tokio::spawn(async move {
            context1.insert_block(Block { block_info: crate::block::BlockInfo { piece_index: 0, begin: 0 }, data: vec![] })?;
            Ok::<(), TorrusError>(())
        });

        let handle2 = tokio::spawn(async move  {
            context2.match_bitfield_len(Bitfield::new().len())?;
            Ok::<(), TorrusError>(())
        });

        let (res1,res2) = futures::try_join!(handle1,handle2).unwrap();

        res1?;
        res2?;
        Ok(())
    }
}
