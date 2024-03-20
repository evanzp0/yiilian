use std::{
    collections::HashSet,
    fmt::Display,
    net::IpAddr,
    ops::Add,
    sync::{Arc, RwLock},
    time::Duration,
};

use chrono::{DateTime, Utc};
use derivative::Derivative;
use tokio::time::sleep;

use crate::common::{expect_log::ExpectLog, shutdown::{spawn_with_shutdown, ShutdownReceiver}};

#[derive(Debug, Clone)]
pub struct BlockList {
    name: String,
    max_size: usize,
    addr_list: Arc<RwLock<HashSet<BlockAddr>>>,
    shutdown_rx: ShutdownReceiver,
}

impl BlockList {
    pub fn new(
        name: &str,
        max_size: usize,
        addr_list: Option<HashSet<BlockAddr>>,
        shutdown_rx: ShutdownReceiver,
    ) -> Self {
        let addr_list = if let Some(val) = addr_list {
            val
        } else {
            HashSet::new()
        };

        BlockList {
            name: name.to_owned(),
            max_size,
            addr_list: Arc::new(RwLock::new(addr_list)),
            shutdown_rx,
        }
    }

    /// 定时清除到期的 block_item
    pub fn prune_loop(&self) {
        let addr_list = self.addr_list.clone();

        spawn_with_shutdown(
            self.shutdown_rx.clone(),
            async move {
                let addr_list = addr_list;

                loop {
                    let now = Utc::now();
                    let mut removable: Vec<BlockAddr> = vec![];
                    for item in addr_list.write().expect_error("block_list read() error").iter()
                    {
                        match item.until {
                            BlockUntil::Infinite => {}
                            BlockUntil::Time(expire_time) => {
                                if now >= expire_time {
                                    removable.push(item.clone())
                                }
                            }
                        }
                    }

                    for item in removable {
                        addr_list.write().expect_error("block_list read() error").remove(&item);
                    }

                    sleep(Duration::from_secs(1)).await;
                }
            },
            format!("{} prune loop", self.name),
            None,
        );
    }

    /// 如果 port 为 -1 则 只判断 ip
    pub fn contains(&self, ip: IpAddr, port: u16) -> bool {
        self.addr_list.read().expect_error("block_list read() error")
            .iter()
            .any(|item| {
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

    pub fn insert(&self, ip: IpAddr, port: i32, duration: Option<Duration>) -> bool {
        if self.len() < (self.max_size as usize) {
            self.addr_list.write().expect_error("block_list read() error")
                .insert(BlockAddr::new(ip, port, duration))
        } else {
            false
        }
    }

    pub fn len(&self) -> usize {
        self.addr_list.read().expect_error("block_list read() error").len()
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

    #[derivative(Hash = "ignore", PartialEq = "ignore")]
    pub until: BlockUntil,
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

        BlockAddr { ip, port, until }
    }
}

#[cfg(test)]
mod tests {

    use std::net::SocketAddr;

    use crate::common::shutdown::create_shutdown;

    use super::*;

    #[tokio::test]
    async fn test_blacklist() {
        let (mut _shutdown_tx, shutdown_rx) = create_shutdown();
        
        let block_list = BlockList::new("test", 1, None, shutdown_rx);
        let addr: SocketAddr = "192.168.1.1:8080".parse().unwrap();

        block_list.insert(addr.ip(), addr.port() as i32, Some(Duration::from_secs(2)));
        block_list.prune_loop();
        assert_eq!(1, block_list.len());
        assert_eq!(true, block_list.contains(addr.ip(), addr.port()));

        sleep(Duration::from_secs(5)).await;
        assert_eq!(0, block_list.len());
    }
}
