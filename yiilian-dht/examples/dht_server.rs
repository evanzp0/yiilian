use std::net::SocketAddr;

use yiilian_core::{
    common::{error::Error, shutdown::create_shutdown, working_dir::WorkingDir},
    service::{FirewallLayer, LogLayer},
};
use yiilian_dht::dht::DhtBuilder;

#[tokio::main]
async fn main() -> Result<(), Error> {
    setup_log();

    let (mut shutdown_tx, shutdown_rx) = create_shutdown();

    let local_addr: SocketAddr = "0.0.0.0:6578".parse().unwrap();

    let wd = WorkingDir::new();

    let dht = DhtBuilder::new(local_addr, shutdown_rx.clone(), Some(100), wd.home_dir())
        .layer(FirewallLayer::new(1000, 20, 1000, shutdown_rx.clone()))
        .layer(LogLayer)
        .build()
        .unwrap();

    drop(shutdown_rx);

    tokio::select! {
        _ = dht.run_loop() => (),
        _ = tokio::signal::ctrl_c() => {
            drop(dht);

            shutdown_tx.shutdown().await;

            println!("\nCtrl + c shutdown");
        },
    }

    Ok(())
}

fn setup_log() {
    dotenv::dotenv().ok();
    env_logger::init();
}
