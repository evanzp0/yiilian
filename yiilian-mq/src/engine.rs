use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use std::time::Duration;
use tokio::time::sleep;
use yiilian_core::common::{
    error::Error,
    shutdown::{spawn_with_shutdown, ShutdownReceiver},
};

use crate::{
    message::{in_message::InMessage, Message},
    topic::Topic,
};

#[derive(Debug)]
pub struct Engine {
    log_data_size: usize,
    path: PathBuf,
    topics: HashMap<String, Arc<Mutex<Topic>>>,
}

impl Engine {
    pub fn new(log_data_size: usize, shutdown_rx: ShutdownReceiver) -> Result<Self, Error> {
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

                    topics.insert(topic_name.to_owned(), Arc::new(Mutex::new(topic)));
                }
            }
        }

        let topic_list: Vec<Arc<Mutex<Topic>>> = topics.values().map(|v| v.clone()).collect();

        spawn_with_shutdown(
            shutdown_rx,
            async move { Engine::purge_loop(topic_list).await },
            "mq engine purge loop",
            None,
        );

        Ok(Engine { path, topics, log_data_size })
    }

    async fn purge_loop(topic_list: Vec<Arc<Mutex<Topic>>>) {
        loop {
            for topic in &topic_list {
                topic.lock().expect("lock topic").purge_segment();
            }

            sleep(Duration::from_secs(60)).await;
        }
    }

    pub fn open_topic(&mut self, topic_name: &str) -> Result<Arc<Mutex<Topic>>, Error> {
        if self.topics.contains_key(topic_name) {
            let topic = self.topics.get(topic_name).expect("get topic").clone();
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
            .insert(topic_name.to_owned(), Arc::new(Mutex::new(topic)));

        Ok(self.topics.get(topic_name).unwrap().clone())
    }

    pub fn remove_topic(&mut self, topic_name: &str) -> Result<(), Error> {
        if self.topics.contains_key(topic_name) {
            self.topics.remove(topic_name);

            let topic_path = {
                let mut p = self.path.clone();
                p.push(topic_name);
                p
            };

            if let Err(_) = fs::remove_dir_all(topic_path.clone()) {
                Err(Error::new_file(
                    None,
                    Some(format!("remove topic {:?} error", topic_path)),
                ))?
            };
        }

        Ok(())
    }

    pub fn push_message(&self, topic_name: &str, message: InMessage) -> Result<(), Error> {
        if let Some(topic) = self.topics.get(topic_name) {
            topic.lock().expect("lock topic").push_message(message)
        } else {
            Err(Error::new_general("Not found topic"))
        }
    }

    pub fn poll_message(&self, topic_name: &str, consumer_name: &str) -> Option<Message> {
        if let Some(topic) = self.topics.get(topic_name) {
            let message = topic
                .lock()
                .expect("lock topic")
                .poll_message(consumer_name);

            message
        } else {
            None
        }
    }
}
