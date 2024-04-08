
use yiilian_mq::{engine::Engine, segment::LOG_DATA_SIZE};

#[tokio::main]
async fn main() {
    let mut dl_path = home::home_dir().unwrap();
    dl_path.push(".yiilian/dl");

    let topic_name = "info_index";
    let mut engine = {
        let mut engine = Engine::new(LOG_DATA_SIZE).expect("create mq engine");
        engine
            .open_topic(topic_name)
            .expect("open info_index topic");

        engine
    };

    for entry in std::fs::read_dir(dl_path.clone()).unwrap() {
        let entry = entry.unwrap();
        let metadata = entry.metadata().unwrap();

        if !metadata.is_dir() {
            continue;
        }

        let entry_name = entry.file_name();
        let entry_path = {
            let mut p = dl_path.clone();
            p.push(entry_name);
            p
        };

        for sub_entry in std::fs::read_dir(entry_path).unwrap() {
            let sub_entry = sub_entry.unwrap();
            let sub_metadata = sub_entry.metadata().unwrap();

            if !sub_metadata.is_file() {
                continue;
            }

            let sub_entry_path = sub_entry.path().to_str().unwrap().to_owned();
            let message = yiilian_mq::message::in_message::InMessage(sub_entry_path.into());

            engine.push_message(topic_name, message).unwrap();
        }
    }
}