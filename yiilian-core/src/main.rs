use yiilian_core::{net::server::Server, filter::{dummy_fiulter::DummyFilter, log_filter::LogFilter}, common::error::{Error, hook_panic}};

#[tokio::main]
async fn main() -> Result<(), Error> {
    setup_log();
    hook_panic();
    
    let ctx_index = 0;
    let svc = DummyFilter;
    let svc = LogFilter::new(ctx_index, svc);
    let server = Server::bind(ctx_index, "0.0.0.0:6578", svc)?;

    let _rst = server.await;

    Ok(())
}

fn setup_log() {
    dotenv::dotenv().ok();
    env_logger::init();
}