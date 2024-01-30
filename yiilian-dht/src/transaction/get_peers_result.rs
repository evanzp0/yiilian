use std::net::SocketAddr;

use bytes::Bytes;
use derivative::Derivative;

use crate::{common::Id, routing_table::Node};


/// Represents the results of a get_peers operation
#[derive(Debug)]
pub struct GetPeersResult {
    info_hash: Id,
    peers: Vec<SocketAddr>,
    responders: Vec<GetPeersResponder>,
}

impl GetPeersResult {
    pub fn new(
        info_hash: Id,
        peers: Vec<SocketAddr>,
        mut responders: Vec<GetPeersResponder>,
    ) -> GetPeersResult {
        responders.sort_unstable_by(|a, b| {
            let a_dist = a.node.id.xor(&info_hash);
            let b_dist = b.node.id.xor(&info_hash);

            a_dist.partial_cmp(&b_dist).unwrap()
        });
        GetPeersResult {
            info_hash,
            peers,
            responders,
        }
    }

    /// The info_hash of the torrent that get_peers was attempting to get peers for
    pub fn info_hash(self) -> Id {
        self.info_hash
    }

    /// Vector full of any peers that were found for the info_hash
    pub fn peers(&self) -> &Vec<SocketAddr> {
        &self.peers
    }

    /// Vector of information about the DHT nodes that responded to get_peers
    ///
    /// This is sorted by distance of the Node to the info_hash, from nearest to farthest.
    pub fn responders(&self) -> &Vec<GetPeersResponder> {
        &self.responders
    }
}

/// Represents the response of a node to a get_peers request, including its Id, IP address,
/// and the token it replied with. This is helpful in case we want to follow up with
/// an announce_peer request.
#[derive(Derivative)] 
#[derivative(Debug, Hash, PartialEq, Eq)]
pub struct GetPeersResponder {
    node: Node,
    #[derivative(Hash="ignore", PartialEq="ignore")] 
    token: Bytes,
}

impl GetPeersResponder {
    pub fn new(node: Node, token: Bytes) -> GetPeersResponder {
        GetPeersResponder { node, token }
    }

    pub fn node(&self) -> &Node {
        &self.node
    }

    pub fn token(&self) -> &Bytes {
        &self.token
    }
}
