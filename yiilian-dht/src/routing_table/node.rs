use std::net::SocketAddr;

use chrono::{Utc, DateTime};
use derivative::Derivative;

use crate::common::id::Id;

#[derive(Derivative)] 
#[derivative(Debug, PartialEq, Eq, Clone, Hash)]
/// Represents a Node on the DHT network
pub struct Node {
    pub id: Id,

    pub address: SocketAddr,

    #[derivative(PartialEq="ignore", Hash="ignore")] 
    /// 我方首次在网络上发现该节点的时间
    pub first_seen: DateTime<Utc>,

    #[derivative(PartialEq="ignore", Hash="ignore")] 
    /// 我方最近在网络上发现该节点的时间
    pub last_seen: DateTime<Utc>,

    #[derivative(PartialEq="ignore", Hash="ignore")] 
    /// 该节点最近响应我方请求的时间
    pub last_verified: Option<DateTime<Utc>>,
}

impl Node {
    /// Creates a new Node from an id and socket address.
    pub fn new(id: Id, address: SocketAddr) -> Node {
        let now = Utc::now();
        Node { 
            id, 
            address,
            first_seen: now,
            last_seen: now,
            last_verified: None, 
        }
    }

    pub fn get_first_seen(&self) -> DateTime<Utc> {
        self.first_seen
    }
}