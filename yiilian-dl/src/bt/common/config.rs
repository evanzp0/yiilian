#![allow(dead_code)]

use std::{net::IpAddr, fs, collections::HashSet};

use serde::{Deserialize, Serialize};
use yiilian_core::{net::block_list::BlockAddr, common::util::atoi};

pub const DEFAULT_CONFIG_FILE: &str = "yiilian-dl.yml";

#[derive(Deserialize, Default, Debug)]
pub struct DlConfig {
    pub bt: BtConfig,
}

impl DlConfig {
    
    pub fn new(bt: BtConfig) -> Self {
        DlConfig {
            bt
        }
    }

    pub fn from_file(cfg_file: &str) -> Self {
        let cfg = fs::read_to_string(cfg_file).expect(&format!("read {} error", cfg_file));
        serde_yaml::from_str(&cfg).expect(&format!("parse {} error", cfg_file))
    }
}

#[derive(Deserialize, Default, Debug)]
pub struct BtConfig {
    pub dht: DhtConfig,
    pub download_port: u16,
}

impl BtConfig {
    pub fn new(dht: DhtConfig, download_port: u16) -> Self {
        BtConfig {
            dht,
            download_port
        }
    }

    pub fn get_dht_block_list(&self) -> Option<HashSet<BlockAddr>> {
        if let Some(block_ips) = &self.dht.block_ips {
            let rst = block_ips.iter().map(|item| {
                let tmp: Vec<&str> = item.split(":").collect();
                let ip: IpAddr = tmp.get(0).unwrap().parse().expect("black_list config parse error");
                let port = if tmp.len() == 2 {
                    atoi(tmp.get(1).unwrap().as_bytes()).expect("black_list config parse error")
                } else {
                    -1
                };
    
                BlockAddr::new(ip, port, None)
            })
            .collect();
            Some(rst)
        } else {
            None
        }

    }
}

#[derive(Default, Deserialize, Serialize, Debug)]
pub struct DhtConfig {
    pub routers: Option<Vec<String>>,
    pub block_ips: Option<Vec<String>>,
    pub port: u16,
    pub workers: Option<usize>,
    pub firewall: Option<FirewallConfig>,
}

#[derive(Default, Deserialize, Serialize, Debug)]
pub struct FirewallConfig {
    pub max_trace: Option<usize>,
    pub max_block: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::{DlConfig, DEFAULT_CONFIG_FILE};

    #[test]
    fn test() {
        let config = DlConfig::from_file(DEFAULT_CONFIG_FILE);

        println!("{:?}", config)
    }
}