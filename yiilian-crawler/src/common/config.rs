#![allow(dead_code)]

use std::{collections::HashSet, fs, net::IpAddr, path::PathBuf};

use serde::{Deserialize, Serialize};
use yiilian_core::{net::block_list::BlockAddr, common::util::atoi};
use yiilian_dl::bt::common::BtConfig;

#[derive(Deserialize, Default, Debug)]
pub struct Config {
    pub dht_cluster: DhtClusterConfig,
    pub bt: BtConfig,
}

impl Config {
    pub fn new() -> Self {
        Config::default()
    }

    pub fn from_file(cfg_file: PathBuf) -> Self {
        let cfg = fs::read_to_string(cfg_file).expect("read cfg_file error");
        serde_yaml::from_str(&cfg).expect("parsecfg_file error")
    }

    pub fn get_dht_block_list(&self) -> Option<HashSet<BlockAddr>> {
        if let Some(block_ips) = &self.dht_cluster.block_ips {
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
pub struct DhtClusterConfig {
    pub routers: Option<Vec<String>>,
    pub block_ips: Option<Vec<String>>,
    pub ports: Vec<u16>,
    pub workers: Option<usize>,
    pub firewall: Option<FirewallConfig>,
}

#[derive(Default, Deserialize, Serialize, Debug)]
pub struct FirewallConfig {
    pub max_trace: Option<usize>,
    pub max_block: Option<usize>,
}