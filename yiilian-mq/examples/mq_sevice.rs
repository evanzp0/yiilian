use std::time::Duration;

use yiilian_core::common::shutdown::create_shutdown;
use yiilian_mq::engine::Engine;

#[tokio::main]
async fn main() {
    let mq_path = home::home_dir().unwrap().join(".yiilian/mq/");

    let (mut shutdown_tx, shutdown_rx) = create_shutdown();

    let mut engine = Engine::new(mq_path, shutdown_rx).unwrap();
    let topic = engine.open_topic("info_hash").unwrap();
    // for i in 0..5 {
    //     let value = format!("value_{}", i);
    //     let message = yiilian_mq::message::in_message::InMessage(value.into());
    //     engine.push_message("info_hash", message).unwrap();
    // }

    // topic.lock().unwrap().consumer_offsets().set("client_4", 12);
    // topic.lock().unwrap().consumer_offsets().set("client_3", 28);
    // topic.lock().unwrap().consumer_offsets().remove("client_4");

    // let message = topic.lock().unwrap().poll_message("client_4");

    let message = engine.poll_message("info_hash", "client_4");
    println!("message: {:?}", message);
    // println!(
    //     "segment_offsets: {:?}",
    //     topic.lock().unwrap().segment_offsets()
    // );
    println!(
        "customer_offsets: {:?}",
        topic.lock().unwrap().consumer_offsets()
    );

    // topic.lock().unwrap().purge_segment();

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            shutdown_tx.shutdown().await;
            println!("\nCtrl + c shutdown");
        },
        _= tokio::spawn(async move {
                let mut i = 0;
                loop {
                    let value = format!("value_{}", i);
                    let message = yiilian_mq::message::in_message::InMessage(value.into());
                    engine.push_message("info_hash", message).unwrap();

                    i += 1;

                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            })
         => {},
    }
}
