use yiilian_core::{
    common::error::{hook_panic, Error},
    data::{Request, Response}, service::util::service_fn,
};
use yiilian_raw::{net::{service::{dummy_service::DummyService, log_service::LogService}, server::Server}, data::raw_body::RawBody};

#[tokio::main]
async fn main() {
    setup_log();
    hook_panic();

    let ctx_index = 0;
    let _svc = DummyService;
    let svc = service_fn(|req: Request<RawBody>| async move {
        println!("{:?}", req);
        Ok::<Response<RawBody>, Error>(Response::new(
            // RawBody::from_str("aaa123"),
            req.body,
            req.remote_addr,
            req.local_addr,
        ))
    });
    let svc = LogService::new(ctx_index, svc);
    let server = Server::bind(ctx_index, "0.0.0.0:6578", svc).unwrap();

    server.run_loop().await;
}

fn setup_log() {
    dotenv::dotenv().ok();
    env_logger::init();
}