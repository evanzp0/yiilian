use std::net::SocketAddr;

use yiilian_core::{
    common::{error::Error, shutdown::create_shutdown},
    service::LogLayer,
};
use yiilian_dht::{
    dht::DhtBuilder,
    service::FirewallLayer,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    setup_log();

    let (mut shutdown_tx, shutdown_rx) = create_shutdown();

    let local_addr: SocketAddr = "0.0.0.0:6578".parse().unwrap();

    let dht = DhtBuilder::new(local_addr, shutdown_rx.clone())
        .layer(FirewallLayer::new(local_addr, 1000, 20))
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
