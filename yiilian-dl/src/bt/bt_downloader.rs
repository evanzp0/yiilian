use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;

use crate::bt::common::BtConfig;
use crate::bt::peer_wire::PeerWire;
use bytes::Bytes;
use hex::ToHex;
use rand::thread_rng;
use yiilian_core::common::error::Error;
use yiilian_core::common::shutdown::ShutdownReceiver;
use yiilian_core::data::{BencodeData, Encode};
use yiilian_core::service::{FirewallLayer, FirewallService};
use yiilian_dht::common::{Id, SettingsBuilder, ID_SIZE};
use yiilian_dht::dht::Dht;
use yiilian_dht::dht::DhtBuilder;
use yiilian_dht::service::RouterService;

pub struct BtDownloader {
    dht: Dht<FirewallService<RouterService>>,
    local_id: Bytes,
    download_dir: PathBuf,
}

impl BtDownloader {
    pub fn new(
        config: &BtConfig,
        download_dir: PathBuf,
        shutdown_rx: ShutdownReceiver,
    ) -> Result<Self, Error> {
        let dht = create_dht(&config, shutdown_rx)?;
        let local_id = Id::from_random(&mut thread_rng()).get_bytes();

        Ok(BtDownloader {
            dht,
            local_id,
            download_dir,
        })
    }

    pub async fn run_loop(&self) {
        self.dht.run_loop().await
    }

    pub async fn fetch_meta(
        &self,
        info_hash: &[u8; ID_SIZE],
    ) -> Result<Option<BTreeMap<Bytes, BencodeData>>, Error> {
        let rst = self.dht.get_peers(Id::new(*info_hash)).await?;

        for peer in rst.peers() {
            let peer_wire = PeerWire::new();
            match peer_wire.fetch_info(*peer, info_hash, &self.local_id).await {
                Ok(info) => return Ok(Some(info)),
                Err(error) => {
                    println!("{:?}", error);
                }
            };
        }

        Ok(None)
    }

    pub async fn fetch_meta_from_target(
        &self,
        target_addr: SocketAddr,
        info_hash: &[u8; ID_SIZE],
    ) -> Result<Option<BTreeMap<Bytes, BencodeData>>, Error> {
        let peer_wire = PeerWire::new();

        match peer_wire
            .fetch_info(target_addr, info_hash, &self.local_id)
            .await
        {
            Ok(info) => return Ok(Some(info)),
            Err(error) => {
                println!("{:?}", error);
            }
        };

        Ok(None)
    }

    pub async fn download_meta_from_target(
        &self,
        target_addr: SocketAddr,
        info_hash: &[u8; ID_SIZE],
    ) -> Result<Option<[u8; ID_SIZE]>, Error> {
        if let Ok(Some(info)) = self.fetch_meta_from_target(target_addr, info_hash).await {
            let torrent = info.encode();
            let mut path = self.download_dir.clone();
            let info_str: String = info_hash.encode_hex();
            path.push(info_str + ".torrent");

            let mut f =
                File::create(path).map_err(|error| Error::new_file(Some(error.into()), None))?;

            f.write_all(&torrent)
                .map_err(|error| Error::new_file(Some(error.into()), None))?;

            Ok(Some(*info_hash))
        } else {
            Ok(None)
        }
    }
    
    pub async fn download_meta(
        &self,
        info_hash: &[u8; ID_SIZE],
    ) -> Result<Option<[u8; ID_SIZE]>, Error> {
        if let Ok(Some(info)) = self.fetch_meta(info_hash).await {
            let torrent = info.encode();
            let mut path = self.download_dir.clone();
            let info_str: String = info_hash.encode_hex();
            path.push(info_str + ".torrent");

            let mut f =
                File::create(path).map_err(|error| Error::new_file(Some(error.into()), None))?;

            f.write_all(&torrent)
                .map_err(|error| Error::new_file(Some(error.into()), None))?;

            Ok(Some(*info_hash))
        } else {
            Ok(None)
        }
    }
}

fn create_dht(
    config: &BtConfig,
    shutdown_rx: ShutdownReceiver,
) -> Result<Dht<FirewallService<RouterService>>, Error> {
    let port = &config.dht.port;
    let block_ips = config.get_dht_block_list();
    let workers = config.dht.workers;

    let settings = if let Some(routers) = &config.dht.routers {
        let mut st = SettingsBuilder::new().build();
        st.routers = routers.clone();
        Some(st)
    } else {
        None
    };

    let (firewall_max_trace, firewall_max_block) = {
        if let Some(firewall_config) = &config.dht.firewall {
            (
                firewall_config.max_trace.unwrap_or(500),
                firewall_config.max_block.unwrap_or(1000),
            )
        } else {
            (500, 1000)
        }
    };

    let local_addr: SocketAddr = format!("0.0.0.0:{port}").parse().unwrap();

    let dht = DhtBuilder::new(local_addr, shutdown_rx.clone(), workers)
        .block_list(block_ips.clone())
        .settings(settings.clone())
        .layer(FirewallLayer::new(
            firewall_max_trace,
            20,
            firewall_max_block,
            shutdown_rx,
        ))
        .build()
        .unwrap();

    Ok(dht)
}
