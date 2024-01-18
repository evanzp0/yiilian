use yiilian_core::{udp::server::Server, filter::{dummy_fiulter::DummyFilter, log_filter::LogFilter}, common::error::Error};



#[tokio::main]
async fn main() -> Result<(), Error> {
    setup_log();
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