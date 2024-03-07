use std::{collections::HashMap, fs, path::PathBuf};

use yiilian_core::common::error::Error;

use crate::topic::Topic;

pub struct Engine {
    path: PathBuf,
    topics: HashMap<String, Topic>,
}

impl Engine {
    pub fn new(path: PathBuf) -> Result<Self, Error> {
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

                    let topic = Topic::new(topic_name, topic_path)?;

                    topics.insert(topic_name.to_owned(), topic);
                }
            }
        }

        Ok(Engine { path, topics })
    }

    pub fn open_topic(&mut self, topic_name: &str) -> Result<&Topic, Error> {
        if self.topics.contains_key(topic_name) {
            let topic = self.topics.get(topic_name).unwrap();
            return Ok(topic);
        }

        let topic_path = {
            let mut p = self.path.clone();
            p.push(topic_name);
            p
        };

        fs::create_dir_all(topic_path.clone())
            .map_err(|error| Error::new_file(Some(error.into()), None))?;

        let topic = Topic::new(topic_name, topic_path)?;

        self.topics.insert(topic_name.to_owned(), topic);

        Ok(self.topics.get(topic_name).unwrap())
    }
}
