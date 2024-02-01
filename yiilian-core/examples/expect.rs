use yiilian_core::common::expect_log::ExpectLog;

fn main() {
    setup_log();

    let a: Option<i32> = None;
    a.expect_error("a is none");
}

fn setup_log() {
    dotenv::dotenv().ok();
    env_logger::init();
}
