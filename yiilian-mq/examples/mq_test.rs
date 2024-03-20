
use yiilian_core::common::shutdown::create_shutdown;
use yiilian_mq::engine::Engine;

#[tokio::main]
async fn main() {
    let (mut _shutdown_tx, shutdown_rx) = create_shutdown();

    let mut engine = Engine::new(shutdown_rx).unwrap();
    let topic = engine.open_topic("info_hash").unwrap();

    println!("{:?}", topic.lock().unwrap().consumer_offsets());

    let message = engine.poll_message("info_hash", "download_meta_client");
    println!("message: {:?}", message);
    println!(
        "customer_offsets: {:?}",
        topic.lock().unwrap().consumer_offsets()
    );

    // topic.lock().unwrap().purge_segment();

    // tokio::select! {
    //     _ = tokio::signal::ctrl_c() => {
    //         shutdown_tx.shutdown().await;
    //         println!("\nCtrl + c shutdown");
    //     },
    //     _= tokio::spawn(async move {
    //             let mut i = 0;
    //             loop {
    //                 let value = format!("value_{}", i);
    //                 let message = yiilian_mq::message::in_message::InMessage(value.into());
    //                 engine.push_message("info_hash", message).unwrap();

    //                 i += 1;

    //                 tokio::time::sleep(Duration::from_secs(5)).await;
    //             }
    //         })
    //      => {},
    // }
}
