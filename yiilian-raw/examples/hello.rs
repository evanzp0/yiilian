use yiilian_core::{
    common::{error::{hook_panic, Error}, shutdown::create_shutdown},
    data::{Request, Response, Body}, service::{service_fn, ServiceBuilder},
};
use yiilian_raw::{data::raw_body::RawBody, service::log_service::LogLayer, net::server::Server};

#[tokio::main]
async fn main() {
    setup_log();
    hook_panic();

    let hello_service = service_fn(|mut req: Request<RawBody>| async move {
        let data = req.body.get_data();
        let s = String::from_utf8_lossy(&data);
        let s = {
            let mut tmp = "hello ".to_owned() + &s;
            if &tmp[tmp.len() - 1 ..] != "\n" {
                tmp += "\n";
            }
            tmp
        };

        Ok::<Response<RawBody>, Error>(Response::new(
            RawBody::from_str(&s),
            req.remote_addr,
            req.local_addr,
        ))
    });
    let svc = ServiceBuilder::new()
        .layer(LogLayer)
        .service(hello_service);

    let (mut shutdown_tx, shutdown_rx) = create_shutdown();
    let server = Server::bind("0.0.0.0:6578", svc, shutdown_rx).unwrap();
    
    tokio::select! {
        _ = server.run_loop() => (),
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