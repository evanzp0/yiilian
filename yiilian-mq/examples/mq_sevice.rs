use yiilian_mq::{engine::Engine, message::in_message::InMessage};

fn main() {
    let mq_path = home::home_dir()
        .unwrap()
        .join(".yiilian/mq/");

    let mut engine = Engine::new(mq_path).unwrap();
    engine.open_topic("info_hash").unwrap();

    for i in 0..5 {
        let value = format!("value_{}", i);
        let message = InMessage(value.into());
        engine.push_message("info_hash", message).unwrap();
    }
}