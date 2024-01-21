use yiilian_core::{
    common::error::{hook_panic, Error},
    data::{Request, Response, Body}, service::{service_fn, ServiceBuilder},
};
use yiilian_raw::{net::{service::log_service::LogLayer, server::Server}, data::raw_body::RawBody};

#[tokio::main]
async fn main() {
    setup_log();
    hook_panic();

    let ctx_index = 0;
    let hello_service = service_fn(|mut req: Request<RawBody>| async move {
        let data = req.body.data();
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
        .layer(LogLayer::new(ctx_index))
        .service(hello_service);
    let server = Server::bind(ctx_index, "0.0.0.0:6578", svc).unwrap();

    server.run_loop().await;
}

fn setup_log() {
    dotenv::dotenv().ok();
    env_logger::init();
}