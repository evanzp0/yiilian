use yiilian_mq::engine::Engine;


fn main() {
    let mq_path = home::home_dir()
        .unwrap()
        .join(".yiilian/mq/");

    let mut engine = Engine::new(mq_path).unwrap();
    engine.open_topic("info_hash").unwrap();
}