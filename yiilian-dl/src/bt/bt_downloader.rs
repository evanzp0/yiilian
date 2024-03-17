use std::collections::BTreeMap;
use std::net::SocketAddr;

use crate::bt::common::BtConfig;
use crate::bt::peer_wire::PeerWire;
use bytes::Bytes;
use rand::thread_rng;
use yiilian_core::common::error::Error;
use yiilian_core::common::shutdown::ShutdownReceiver;
use yiilian_core::data::BencodeData;
use yiilian_core::service::{FirewallLayer, FirewallService};
use yiilian_dht::common::{Id, SettingsBuilder};
use yiilian_dht::dht::Dht;
use yiilian_dht::dht::DhtBuilder;
use yiilian_dht::service::RouterService;

pub struct BtDownloader {
    dht: Dht<FirewallService<RouterService>>,
    local_id: Bytes,
}

impl BtDownloader {
    pub fn new(
        config: BtConfig,
        shutdown_rx: ShutdownReceiver,
    ) -> Result<Self, Error> {
        let dht = create_dht(&config, shutdown_rx.clone())?;
        let local_id = Id::from_random(&mut thread_rng()).get_bytes();

        Ok(BtDownloader { dht, local_id })
    }

    pub async fn download_meta(&self, info_hash: &[u8],) -> Result<Option<BTreeMap<Bytes, BencodeData>>, Error> {
        let rst = self.dht.get_peers(info_hash.try_into()?).await?;

        for peer in rst.peers() {
            let peer_wire = PeerWire::new();
            let info = peer_wire
                .fetch_info(*peer, info_hash, &self.local_id)
                .await?;

            return Ok(Some(info))
        }

        Ok(None)
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
            shutdown_rx.clone(),
        ))
        .build()
        .unwrap();

    Ok(dht)
}
