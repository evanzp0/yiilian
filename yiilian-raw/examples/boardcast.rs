use std::fmt::Debug;

use tokio::sync::broadcast::{self, error::RecvError, Receiver};
use yiilian_core::{
    common::{
        error::{hook_panic, Error},
        shutdown::create_shutdown,
    },
    data::{Body, Request, Response},
    service::{service_fn, EventLayer, ServiceBuilder},
};
use yiilian_raw::{data::raw_body::RawBody, net::server::Server, service::log_service::LogLayer};

#[tokio::main]
async fn main() {
    setup_log();
    hook_panic();

    let echo_service = service_fn(|mut req: Request<RawBody>| async move {
        let data = req.body.get_data();
        let s = String::from_utf8_lossy(&data);

        Ok::<Response<RawBody>, Error>(Response::new(
            RawBody::from_str(&s),
            req.remote_addr,
            req.local_addr,
        ))
    });

    let (tx, rx) = broadcast::channel(1);
    let svc = ServiceBuilder::new()
        .layer(LogLayer)
        .layer(EventLayer::new(tx))
        .service(echo_service);

    let (mut shutdown_tx, shutdown_rx) = create_shutdown();
    let server = Server::bind("0.0.0.0:6578", svc, shutdown_rx).unwrap();

    tokio::select! {
        _ = server.run_loop() => (),
        _ = event_loop(rx) => (),
        _ = tokio::signal::ctrl_c() => {
            shutdown_tx.shutdown().await;
            println!("\nCtrl + c shutdown");
        },
    }
}

fn setup_log() {
    dotenv::dotenv().ok();
    env_logger::init();
}

async fn event_loop<T>(mut rx: Receiver<T>) 
where
    T: Debug + Clone,
{
    loop {
        let rst = rx.recv().await;
        match rst {
            Ok(val) => {
                println!("recv: {:?}", val);
            },
            Err(error) => match error {
                RecvError::Closed => {
                    println!("Send closed");
                    break;
                },
                RecvError::Lagged(_) => (),
            },
        }
        
    }
}