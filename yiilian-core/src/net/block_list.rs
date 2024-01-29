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

use crate::{
    common::shutdown::{spawn_with_shutdown, ShutdownReceiver},
    except_result,
};

#[derive(Debug)]
pub struct BlockList {
    max_size: i32,
    addr_list: Arc<RwLock<HashSet<BlockAddr>>>,
    shutdown_rx: ShutdownReceiver,
}

impl BlockList {
    pub fn new(
        max_size: i32,
        addr_list: Option<HashSet<BlockAddr>>,
        shutdown_rx: ShutdownReceiver,
    ) -> Self {
        let addr_list = if let Some(val) = addr_list {
            val
        } else {
            HashSet::new()
        };

        BlockList {
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
                    for item in except_result!(addr_list.write(), "block_list read() error").iter()
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
                        except_result!(addr_list.write(), "block_list read() error").remove(&item);
                    }

                    sleep(Duration::from_secs(1)).await;
                }
            },
            "block_list prune loop",
            None,
        );
    }

    /// 如果 port 为 -1 则 只判断 ip
    pub fn contains(&self, ip: IpAddr, port: u16) -> bool {
        except_result!(self.addr_list.read(), "block_list read() error")
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

    pub fn insert(&mut self, ip: IpAddr, port: i32, duration: Option<Duration>) -> bool {
        if self.len() < (self.max_size as usize) {
            except_result!(self.addr_list.write(), "block_list read() error")
                .insert(BlockAddr::new(ip, port, duration))
        } else {
            false
        }
    }

    pub fn len(&self) -> usize {
        except_result!(self.addr_list.read(), "block_list read() error").len()
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

// #[cfg(test)]
// mod tests {

//     use std::net::SocketAddr;

//     use super::*;

//     #[test]
//     fn test_blacklist() {
//         let mut block_list = BlockList::new(10, None);
//         let addr: SocketAddr = "192.168.1.1:8080".parse().unwrap();

//         block_list.insert(addr.ip(), addr.port() as i32, None);
//         assert_eq!(1, block_list.len());
//         assert_eq!(true, block_list.contains(addr.ip(), addr.port()));

//         assert_eq!(true, block_list.remove(addr.ip(), addr.port() as i32));
//     }
// }
