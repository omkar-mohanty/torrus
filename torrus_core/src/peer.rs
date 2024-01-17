use crate::prelude::{Sha1Hash, ID};
use std::net::IpAddr;

pub struct PeerInfo {
    pub id: ID,
    pub addr: IpAddr,
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
    pub choke: ChokeStatus,
    pub intrest_status: IntrestStatus,
}

impl Default for PeerState {
    fn default() -> Self {
        Self {
            choke: ChokeStatus::Chocked,
            intrest_status: IntrestStatus::NotInterested,
        }
    }
}
