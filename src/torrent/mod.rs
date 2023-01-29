use futures::Stream;
use futures::{future::join_all, FutureExt, SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
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
    pub piece_handler: RwLock<PieceHandler>,
    /// 20 byte infohash of the torrent
    pub info_hash: Hash,
    /// 20 byte client ID
    pub client_id: PeerId,
    /// V1 Bittorrent metainfo
    pub metainfo: Metainfo,
}

impl Context {
    pub fn new(piece_handler: PieceHandler, client_id: PeerId, metainfo: Metainfo) -> Result<Self> {
        let piece_handler = RwLock::new(piece_handler);

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

    fn get_read_handle(&self) -> Result<RwLockReadGuard<PieceHandler>> {
        match self.piece_handler.read() {
            Ok(handler) => Ok(handler),
            Err(err) => {
                log::error!("\tget_read_handle : Error:\t{}", err);
                Err(TorrusError::new(&err.to_string()))
            }
        }
    }

    fn get_write_handle(&self) -> Result<RwLockWriteGuard<PieceHandler>> {
        match self.piece_handler.write() {
            Ok(handler) => Ok(handler),
            Err(err) => {
                log::error!("\tget_write_handle : Error:\t{}", err);
                Err(TorrusError::new(&err.to_string()))
            }
        }
    }

    pub fn match_bitfield_len(&self, len: usize) -> Result<bool> {
        let handler = self.get_read_handle()?;

        Ok(handler.match_bitfield_len(len))
    }

    pub fn insert_block(&self, block: Block) -> Result<()> {
        let mut handler = self.get_write_handle()?;

        Ok(handler.insert_block(block)?)
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

        let context = Context::new(piece_handler, client_id, metainfo)?;

        let context = Arc::new(context);

        let peers = HashMap::new();

        Ok(Self {
            context,
            client_id,
            peers,
            cmd_rcv,
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

    fn get_trackers(&self) -> Result<Vec<Tracker>> {
        let trackers = if let Some(url) = &self.context.metainfo.announce {
            let tracker = Tracker::from_url_string(url, Arc::clone(&self.context))?;

            vec![tracker]
        } else if let Some(al) = &self.context.metainfo.announce_list {
            let mut trackers = Vec::new();

            for a in al {
                let tracker = Tracker::from_url_string(&a[0], Arc::clone(&self.context))?;
                trackers.push(tracker)
            }

            trackers
        } else {
            vec![]
        };

        Ok(trackers)
    }

    /// Start Bittorrent `Handshake` protocol with all the peers and then start the wire protocol.
    pub async fn start(&mut self) -> Result<()> {
        let addr: SocketAddr = "0.0.0.0:0".parse().unwrap();

        let port = addr.port();

        let trackers = self.get_trackers()?;

        let discovery = Discovery::new(trackers, self.client_id);

        let fut = Box::pin(futures::stream::unfold(
            (0, discovery),
            |state| async move {
                let (state, mut discovery) = state;

                if state < 30 {
                    let next_state = 30 - state;

                    let peers = match discovery.get_peers(next_state, port).await {
                        Ok(peers) => peers,
                        Err(_) => vec![],
                    };

                    let next_state = (30 - peers.len()) as i32;

                    Some((peers, (next_state, discovery)))
                } else {
                    None
                }
            },
        ));

        let listner = TcpListener::bind(addr).await?;

        self.handle_events(listner, fut).await?;

        Ok(())
    }

    async fn handle_events(
        &mut self,
        listner: TcpListener,
        mut peer_stream: impl Stream<Item = Vec<PeerAddr>> + Unpin,
    ) -> Result<()> {
        loop {
            tokio::select! {
             cmd = self.cmd_rcv.recv() => {
                    if let Some(command) = cmd {
                    self.handle_command(command);
                    }
                }
             res = listner.accept() => {
                if let Ok((stream, _)) = res {
                       let _ =  self.insert_new_peer_stream(stream);
                    }
                }
             peers = peer_stream.next() => {
                    if let Some(peers) = peers {
                        self.get_peer_streams(peers).await;
                    }
                }
            }
        }
    }

    fn handle_command(&self, command: TorrentCommand) {
        use TorrentCommand::*;

        match command {
            Progress => {
                let have_count = self.context.piece_handler.read().unwrap().have_count();
                let miss_count = self.context.piece_handler.read().unwrap().miss_count();

                log::info!(
                    "\thandle_command Progress:\t{}%",
                    (have_count / miss_count) * 100
                );
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

    if let Err(_) = timeout(timeout_dur, fut).await {
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
    use crate::{new_peer_id, Result};
    use tokio::sync::mpsc::unbounded_channel;

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
