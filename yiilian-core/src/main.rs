use yiilian_core::{net::server::Server, service::{dummy_service::DummyService, log_service::LogService}, common::error::{Error, hook_panic}};

#[tokio::main]
async fn main() -> Result<(), Error> {
    setup_log();
    hook_panic();
    
    let ctx_index = 0;
    let svc = DummyService;
    let svc = LogService::new(ctx_index, svc);
    let server = Server::bind(ctx_index, "0.0.0.0:6578", svc)?;

    let _rst = server.await;

    Ok(())
}

fn setup_log() {
    dotenv::dotenv().ok();
    env_logger::init();
}