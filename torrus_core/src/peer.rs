use crate::prelude::{Sha1Hash, ID};
use std::net::IpAddr;

pub struct PeerInfo {
    id: ID,
    addr: IpAddr,
}

pub trait PeerSource {
    fn get_peers(&mut self) -> impl Iterator<Item = PeerInfo>;
}

#[derive(Clone, Copy, Debug)]
pub enum ChokeStatus {
    NotChocked,
    Chocked,
}

#[derive(Clone, Copy, Debug)]
pub enum IntrestStatus {
    NotInterested,
    Interested,
}

pub struct PeerState {
    choke: ChokeStatus,
    intrest_status: IntrestStatus,
}

impl Default for PeerState {
    fn default() -> Self {
        Self {
            choke: ChokeStatus::Chocked,
            intrest_status: IntrestStatus::NotInterested,
        }
    }
}

pub struct Peer {
    peer_info: PeerInfo,
    state: PeerState,
}

impl Peer {
    pub fn new(peer_info: PeerInfo) -> Self {
        Peer {
            peer_info,
            state: PeerState::default(),
        }
    }

    pub fn choke_status(&self) -> ChokeStatus {
        self.state.choke
    }

    pub fn interest(&self) -> IntrestStatus {
        self.state.intrest_status
    }

    pub fn ip_addr(&self) -> IpAddr {
        self.peer_info.addr
    }
}

impl Sha1Hash for Peer {
    fn as_sha1(&self) -> ID {
        self.peer_info.id
    }
}
