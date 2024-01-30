
use std::{net::SocketAddr, path::Path};

use futures::future::join_all;

use tokio::sync::broadcast::{self, Sender};
use yiilian_core::{
    common::{
        error::Error,
        shutdown::{create_shutdown, ShutdownReceiver},
    },
    data::Request,
    service::EventLayer,
};
use yiilian_crawler::common::{Config, DEFAULT_CONFIG_FILE};
use yiilian_crawler::event::RecvAnnounceListener;
use yiilian_dht::{
    common::SettingsBuilder, data::body::KrpcBody, dht::{Dht, DhtBuilder}, service::KrpcService
};

#[tokio::main]
async fn main() {
    set_up_logging_from_file::<&str>(None);
    let config = Config::from_file(DEFAULT_CONFIG_FILE);
    let (mut shutdown_tx, shutdown_rx) = create_shutdown();
    let (tx, rx) = broadcast::channel(1024);
    let dht_list = create_dht_list(&config, shutdown_rx.clone(), tx).unwrap();
    let mut announce_listener = RecvAnnounceListener::new(rx, shutdown_rx.clone());

    drop(shutdown_rx);

    tokio::select! {
        _  = async {
            let mut futs = vec![];
            for dht in &dht_list {
                println!("Listening at: {:?}", dht.local_addr);
                futs.push(dht.run_loop());
            }
            join_all(futs).await;

        } => (),
        _ = announce_listener.listen() => (),
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
    tx: Sender<Request<KrpcBody>>,
) -> Result<
    Vec<
        Dht<impl KrpcService<KrpcBody, ResBody = KrpcBody, Error = Error> + Clone + Send + 'static>,
    >,
    Error,
> {
    let mut dht_list = vec![];

    let ports = &config.dht.ports;
    let block_ips = config.get_dht_block_list();
    let settings = if let Some(routers) = &config.dht.routers {
        let mut st = SettingsBuilder::new().build();
        st.routers = routers.clone();
        Some(st)
    } else {
        None
    };

    if ports.len() == 2 {
        let port_start = ports[0];
        let port_end = ports[1];
        for port in port_start..=port_end {
            let local_addr: SocketAddr = format!("0.0.0.0:{port}").parse().unwrap();

            let dht = DhtBuilder::new(local_addr, shutdown_rx.clone())
                .block_list(block_ips.clone())
                .settings(settings.clone())
                .layer(EventLayer::new(tx.clone()))
                .build()
                .unwrap();

            dht_list.push(dht);
        }
    } else {
        for port in ports {
            let local_addr: SocketAddr = format!("0.0.0.0:{port}").parse().unwrap();

            let dht = DhtBuilder::new(local_addr, shutdown_rx.clone())
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
