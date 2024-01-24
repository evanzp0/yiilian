use std::{net::SocketAddr, num::NonZeroUsize};

use chrono::{DateTime, Utc};
use lru::LruCache;
use crate::common::id::Id;

use super::Peer;

#[derive(Debug)]
pub struct PeerManager {
    /// LruCache 最近最少使用缓存： key = resource_infohash_id, value = PeeerInfo
    peers: LruCache<Id, LruCache<SocketAddr, Peer>>,
    max_peers_per_resource: usize,
}


impl PeerManager {
    pub fn new(max_resource: usize, max_peers_per_resource: usize) -> PeerManager {
        PeerManager {
            peers: LruCache::new(NonZeroUsize::new(max_resource).unwrap()),
            max_peers_per_resource,
        }
    }

    pub fn announce_peer(&mut self, info_hash: Id, peer_addr: SocketAddr) {
        let peers = &mut self.peers;
        match peers.get_mut(&info_hash) {
            Some(swarm_lru) => {
                swarm_lru.put(peer_addr, Peer::new(peer_addr));
            }

            None => {
                let mut swarm_lru = LruCache::new(NonZeroUsize::new(self.max_peers_per_resource).unwrap());
                swarm_lru.put(peer_addr, Peer::new(peer_addr));
                peers.put(info_hash.clone(), swarm_lru);
            }
        }
        // log::debug!(target: "yiilian_dht::PeerManager", "{} is in swarm with info_hash {}", peer_addr, info_hash);
    }

    /// 返回 最后更新时间 > newer_than 的 IPv4 的 peers 的 IP 地址列表
    pub fn get_peers(
        &mut self,
        info_hash: &Id,
        newer_than: Option<DateTime<Utc>>,
    ) -> Vec<SocketAddr> {
        let infos = self.get_peers_info(info_hash, newer_than);
        infos.iter().map(|info| info.addr).collect()
    }

    /// 返回 最后更新时间 > newer_than 的 IPv4 的 peers
    pub fn get_peers_info(
        &mut self,
        info_hash: &Id,
        newer_than: Option<DateTime<Utc>>,
    ) -> Vec<Peer> {
        let peers = &mut self.peers;
        let mut to_ret = Vec::new();
        if let Some(swarm_lru) = peers.get(info_hash) {
            let mut tmp = swarm_lru
                .iter()
                .filter(|pi| pi.0.ip().is_ipv4()) // Only return IPv4 for now, you dog!
                .filter(|pi| newer_than.is_none() || pi.1.last_updated > newer_than.unwrap())
                .map(|pi| *pi.1)
                .collect();
            to_ret.append(&mut tmp);
        }

        to_ret
    }

    /// 返回所有资源的 info_hash
    pub fn get_info_hashes(&self) -> Vec<Id> {
        self.peers.iter().map(|kv| kv.0.clone()).collect()
    }
}