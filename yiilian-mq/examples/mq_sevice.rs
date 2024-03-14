use yiilian_mq::engine::Engine;

fn main() {
    let mq_path = home::home_dir()
        .unwrap()
        .join(".yiilian/mq/");

    let mut engine = Engine::new(mq_path).unwrap();
    let topic = engine.open_topic("info_hash").unwrap();
    println!("{:?}", topic.lock().unwrap().consumer_offsets());
    // for i in 0..5 {
    //     let value = format!("value_{}", i);
    //     let message = yiilian_mq::message::in_message::InMessage(value.into());
    //     engine.push_message("info_hash", message).unwrap();
    // }

    // topic.lock().unwrap().consumer_offsets().set("client_4", 12);
    // topic.lock().unwrap().consumer_offsets().set("client_3", 24);
    // topic.lock().unwrap().consumer_offsets().remove("client_4");

    // let message = topic.lock().unwrap().poll_message("client_4");

    let message = engine.poll_message("info_hash", "client_4");
    println!("message: {:?}", message);
    println!("segment_offsets: {:?}", topic.lock().unwrap().segment_offsets());
    println!("customer_offsets: {:?}", topic.lock().unwrap().consumer_offsets());

    // topic.lock().unwrap().purge_segment();
}