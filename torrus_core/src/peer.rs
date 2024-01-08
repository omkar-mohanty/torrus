use std::net::IpAddr;

use crate::prelude::{Sha1Hash, ID};

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
    id: ID,
    ip: IpAddr,
    state: PeerState,
}

impl Peer {
    pub fn new(id: ID, ip: IpAddr) -> Self {
        Peer {
            id,
            ip,
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
        self.ip
    }
}

impl Sha1Hash for Peer {
    fn into_sha1(&self) -> ID {
        self.id
    }
}
