use std::net::SocketAddr;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Persist {
    pub node_addrs: Vec<SocketAddr>,
}
