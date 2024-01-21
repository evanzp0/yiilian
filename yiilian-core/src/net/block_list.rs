use std::{net::IpAddr, time::Duration, collections::HashSet, fmt::Display, ops::Add};

use chrono::{DateTime, Utc};
use derivative::Derivative;

#[derive(Debug)]
pub struct BlockList<> {
    max_size: i32,
    addr_list: HashSet<BlockAddr>
}

impl BlockList {
    pub fn new(max_size: i32, addr_list: Option<HashSet<BlockAddr>>) -> Self {
        BlockList { 
            max_size, 
            addr_list: if addr_list.is_some() {
                    addr_list.unwrap()
                } else {
                    HashSet::new() 
                }
        }
    }

    /// 如果 port 为 -1 则 只判断 ip 
    pub fn contains(&self, ip: IpAddr, port: u16) -> bool {
        self.addr_list.iter().any(|item| {
            if item.ip == ip {
                if item.port != -1 {
                    item.port == port as i32
                } else {
                    true
                }
            } else {
                false
            }
        })
    }

    pub fn remove(&mut self, ip: IpAddr, port: i32) -> bool {
        self.addr_list.remove(&BlockAddr::new(ip, port, None))
    }

    pub fn insert(&mut self, ip: IpAddr, port: i32, duration: Option<Duration>) -> bool {
        if self.len() < (self.max_size as usize) {
            self.addr_list.insert(BlockAddr::new(ip, port, duration))
        } else {
            false
        }
    }

    pub fn len(&self) -> usize {
        self.addr_list.len()
    }
    
    pub fn get_addrs(&self) -> &HashSet<BlockAddr> {
        &self.addr_list
    }
}

#[derive(Debug, Clone)]
pub enum BlockUntil {
    Infinite,
    Time(DateTime<Utc>),
}

#[derive(Derivative)] 
#[derivative(Debug, PartialEq, Eq, Clone, Hash)]
pub struct BlockAddr {
    pub ip: IpAddr,
    pub port: i32,

    #[derivative(Hash="ignore", PartialEq = "ignore")] 
    pub until: BlockUntil
}

impl Display for BlockAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ip: {}, untilL {:?}", self.ip, self.until)
    }
}

impl BlockAddr {
    pub fn new(ip: IpAddr, port: i32, duration: Option<Duration>) -> Self {
        let until = {
            if let Some(dur) = duration {
                let now = Utc::now();
                BlockUntil::Time(now.add(dur))
            } else {
                BlockUntil::Infinite
            }
        };

        BlockAddr {
            ip,
            port,
            until
        }
    }
}

#[cfg(test)]
mod tests {

    use std::net::SocketAddr;

    use super::*;

    #[test]
    fn test_blacklist() {
        let mut block_list = BlockList::new(10, None);
        let addr: SocketAddr = "192.168.1.1:8080".parse().unwrap();

        block_list.insert(addr.ip(), addr.port() as i32, None);
        assert_eq!(1, block_list.len());
        assert_eq!(true, block_list.contains(addr.ip(), addr.port()));

        assert_eq!(true, block_list.remove(addr.ip(), addr.port() as i32));
    }
}