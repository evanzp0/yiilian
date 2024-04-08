use std::sync::{Arc, Mutex};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
};

use std::time::Duration;
use tokio::time::sleep;
use yiilian_core::common::error::Error;

use crate::{
    message::{in_message::InMessage, Message},
    topic::Topic,
};

#[derive(Debug)]
pub struct Engine {
    log_data_size: usize,
    path: PathBuf,
    topics: HashMap<String, Topic>,
}

impl Engine {
    pub fn new(log_data_size: usize) -> Result<Self, Error> {
        let path: PathBuf = home::home_dir().unwrap().join(".yiilian/mq/");

        fs::create_dir_all(path.clone())
            .map_err(|error| Error::new_file(Some(error.into()), None))?;

        let dir = path
            .as_path()
            .read_dir()
            .map_err(|error| Error::new_file(Some(error.into()), None))?;
        let mut topics = HashMap::new();
        for x in dir {
            if let Ok(topic_dir) = x {
                let tmp_path = topic_dir.path();

                if tmp_path.is_dir() && tmp_path.file_name().is_some() {
                    let topic_name = tmp_path.file_name().unwrap().to_str().unwrap();
                    let topic_path = {
                        let mut p = path.clone();
                        p.push(tmp_path.clone());
                        p
                    };

                    let topic = Topic::new(topic_name, topic_path, log_data_size)?;

                    topics.insert(topic_name.to_owned(),topic);
                }
            }
        }

        Ok(Engine {
            path,
            topics,
            log_data_size,
        })
    }

    pub fn open_topic(&mut self, topic_name: &str) -> Result<&mut Topic, Error> {
        if self.topics.contains_key(topic_name) {
            let topic = self.topics.get_mut(topic_name).expect("get topic");
            return Ok(topic);
        }

        let topic_path = {
            let mut p = self.path.clone();
            p.push(topic_name);
            p
        };

        fs::create_dir_all(topic_path.clone())
            .map_err(|error| Error::new_file(Some(error.into()), None))?;

        let topic = Topic::new(topic_name, topic_path, self.log_data_size)?;

        self.topics
            .insert(topic_name.to_owned(), topic);

        Ok(self.topics.get_mut(topic_name).expect("get topic"))
    }

    pub fn remove_topic(&mut self, topic_name: &str) {
        if self.topics.contains_key(topic_name) {
            self.topics.remove(topic_name);

            let topic_path = {
                let mut p = self.path.clone();
                p.push(topic_name);
                p
            };

            fs::remove_dir_all(topic_path.clone()).ok();
        }
    }

    pub fn push_message(&mut self, topic_name: &str, message: InMessage) -> Result<(), Error> {
        if let Some(topic) = self.topics.get_mut(topic_name) {
            topic.push_message(message)
        } else {
            Err(Error::new_general("Not found topic"))
        }
    }

    pub fn poll_message(&mut self, topic_name: &str, consumer_name: &str) -> Option<Message> {
        if let Some(topic) = self.topics.get_mut(topic_name) {
            let message = topic.poll_message(consumer_name);

            message
        } else {
            None
        }
    }

    pub fn message_count(&self, topic_name: &str, consumer_name: &str) -> u64 {
        if let Some(topic) = self.topics.get(topic_name) {
            topic.count(consumer_name)
        } else {
            0
        }
    }
}

pub async fn purge_loop(engine: Arc<Mutex<Engine>>) {
    loop {
        let mut engine = engine.lock().expect("lock engine");
        let topic_list: Vec<&mut Topic> = engine.topics.values_mut().map(|v| v).collect();
        for topic in topic_list {
            topic.purge_segment();
        }

        sleep(Duration::from_secs(60)).await;
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_engine() {
        let topic_name = "test_count";
        let consumer_name = "test_client";

        let mut engine = {
            let mut engine = Engine::new(100).expect("create mq engine");
            engine
                .open_topic(topic_name)
                .expect("open test_count topic");

            engine
        };

        for i in 0..20 {
            let value = format!("value_{}", i);
            let message = InMessage(value.into());
            engine.push_message(topic_name, message).unwrap();
        }

        for _i in 0..8 {
            engine.poll_message(topic_name, consumer_name).unwrap();
        }

        let count = engine.message_count(topic_name, consumer_name);
        assert_eq!(12, count);

        engine.remove_topic(topic_name);
    }
}
