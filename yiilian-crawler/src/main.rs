use std::{net::SocketAddr, path::Path, sync::Arc, time::Duration};

use futures::future::join_all;

use tokio::{
    sync::broadcast::{self, Sender},
    time::sleep,
};
use yiilian_core::{
    common::{
        error::Error,
        shutdown::{create_shutdown, ShutdownReceiver},
    },
    data::Request,
    service::{EventLayer, FirewallLayer},
};
use yiilian_crawler::common::{Config, DEFAULT_CONFIG_FILE};
use yiilian_crawler::event::RecvAnnounceListener;
use yiilian_dht::{
    common::SettingsBuilder,
    data::body::KrpcBody,
    dht::{Dht, DhtBuilder},
    service::KrpcService,
};
use yiilian_mq::engine::Engine;

#[tokio::main]
async fn main() {
    set_up_logging_from_file::<&str>(None);
    let config = Config::from_file(DEFAULT_CONFIG_FILE);
    let (mut shutdown_tx, shutdown_rx) = create_shutdown();
    let (tx, rx) = broadcast::channel(1024);
    let dht_list = create_dht_list(&config, shutdown_rx.clone(), tx).unwrap();

    let mq_engine = {
        let mut engine = Engine::new(shutdown_rx.clone()).expect("create mq engine");
        engine
            .open_topic("info_hash")
            .expect("open info_hash topic");

        Arc::new(engine)
    };

    let mut announce_listener = RecvAnnounceListener::new(rx, mq_engine, shutdown_rx.clone());

    drop(shutdown_rx);

    tokio::select! {
        _  = async {
            // loop {
                let mut futs = vec![];
                for dht in &dht_list {
                    println!("Listening at: {:?}", dht.local_addr);
                    futs.push(dht.run_loop());
                }

                join_all(futs).await;
                sleep(Duration::from_secs(10 * 60)).await;
                log::info!("restart dht");
            // }
        } => (),
        _ = async {
            // tokio::spawn(async move {
                announce_listener.listen().await
            // })
        } => (),
        _ = tokio::signal::ctrl_c() => {
            drop(dht_list);

            shutdown_tx.shutdown().await;

            println!("\nCtrl + c shutdown");
        },
    };
}

fn create_dht_list(
    config: &Config,
    shutdown_rx: ShutdownReceiver,
    tx: Sender<Arc<Request<KrpcBody>>>,
) -> Result<
    Vec<Dht<impl KrpcService<KrpcBody, ResBody = KrpcBody, Error = Error> + Clone + Send + 'static>>,
    Error,
> {
    let mut dht_list = vec![];

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

    if ports.len() == 2 {
        let port_start = ports[0];
        let port_end = ports[1];
        for port in port_start..=port_end {
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
                .layer(EventLayer::new(tx.clone()))
                .build()
                .unwrap();

            dht_list.push(dht);
        }
    } else {
        for port in ports {
            let local_addr: SocketAddr = format!("0.0.0.0:{port}").parse().unwrap();

            let dht = DhtBuilder::new(local_addr, shutdown_rx.clone(), workers)
                .block_list(block_ips.clone())
                .settings(settings.clone())
                .layer(FirewallLayer::new(
                    10,
                    20,
                    firewall_max_block,
                    shutdown_rx.clone(),
                ))
                .layer(EventLayer::new(tx.clone()))
                .build()
                .unwrap();

            dht_list.push(dht);
        }
    }

    Ok(dht_list)
}

fn set_up_logging_from_file<P: AsRef<Path>>(file_path: Option<&P>) {
    if let Some(file_path) = file_path {
        log4rs::init_file(file_path, Default::default()).unwrap();
    } else {
        log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    }
}
