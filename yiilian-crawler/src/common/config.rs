#![allow(dead_code)]

use std::{net::IpAddr, fs, collections::HashSet};

use serde::{Deserialize, Serialize};
use yiilian_core::{net::block_list::BlockAddr, common::util::atoi};

pub const DEFAULT_CONFIG_FILE: &str = "yiilian-crawler.yml";

#[derive(Deserialize, Default)]
pub struct Config {
    pub dht: DhtConfig,
}

impl Config {
    pub fn new() -> Self {
        Config::default()
    }

    pub fn from_file(cfg_file: &Option<String>) -> Self {
        let cfg_file = if let Some(cfg_file) = cfg_file {
            cfg_file
        } else {
            DEFAULT_CONFIG_FILE
        };

        let cfg = fs::read_to_string(cfg_file).expect(&format!("read {} error", cfg_file));
        serde_yaml::from_str(&cfg).expect(&format!("parse {} error", cfg_file))
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

#[derive(Default, Deserialize, Serialize)]
pub struct DhtConfig {
    pub routers: Option<Vec<String>>,
    pub block_ips: Option<Vec<String>>,
    pub node_file_prefix: Option<String>,
    pub ports: DhtPorts,
}

#[derive(Deserialize, Serialize)]
pub enum DhtPorts {
    Int(i32),
    Array(Vec<u16>)
}

impl Default for DhtPorts {
    fn default() -> Self {
        DhtPorts::Int(1)
    }
}

#[cfg(test)]
mod tests {
    use super::DhtPorts;

    #[test]
    fn test_ser_de_dht_ports() {
        let ports = DhtPorts::Int(1);
        let s = serde_yaml::to_string(&ports).unwrap();
        assert_eq!("!Int 1\n", s);

        let ports = DhtPorts::Array(vec![123, 4567]);
        let s = serde_yaml::to_string(&ports).unwrap();
        assert_eq!("!Array\n- 123\n- 4567\n", s);
    }
}