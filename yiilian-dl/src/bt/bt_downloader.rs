use std::net::SocketAddr;

use tokio::sync::oneshot::Receiver;
use yiilian_core::common::error::Error;
use yiilian_core::common::shutdown::ShutdownReceiver;
use yiilian_core::service::{FirewallLayer, FirewallService};
use yiilian_dht::common::SettingsBuilder;
use yiilian_dht::dht::DhtBuilder;
use yiilian_dht::service::RouterService;
use yiilian_dht::dht::Dht;
use crate::bt::common::BtConfig;

use super::bt_command::BtCommand;

pub struct BtDownloader {
    dht: Dht<FirewallService<RouterService>>,
    cmd_rx: Receiver<BtCommand>,
}

impl BtDownloader
{
    pub fn new(config: BtConfig, cmd_rx: Receiver<BtCommand>, shutdown_rx: ShutdownReceiver) -> Self {
        let dht = create_dht(&config, shutdown_rx.clone()).unwrap();

        BtDownloader {
            dht,
            cmd_rx
        }
    }
}

fn create_dht(
    config: &BtConfig,
    shutdown_rx: ShutdownReceiver,
) -> Result<
    Dht<FirewallService<RouterService>>,
    Error,
> {
    let ports = &config.dht.ports;
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
    
    if ports.len() >= 1 {
        let port = ports[0];
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
    } else {
        Err(Error::new_general("Bt DHT port not config"))
    }
}