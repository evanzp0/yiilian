use std::net::SocketAddr;

use chrono::{DateTime, Utc};

#[derive(Eq, PartialEq, Copy, Clone)]
pub struct Peer {
    pub addr: SocketAddr,
    pub last_updated: DateTime<Utc>,
}

impl Peer {
    pub fn new(addr: SocketAddr) -> Peer {
        Peer {
            addr,
            last_updated: Utc::now(),
        }
    }
}
