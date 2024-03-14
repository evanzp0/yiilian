use std::{collections::HashMap, fs::OpenOptions, io::{Read, Write}, path::PathBuf};

use yiilian_core::common::error::Error;

#[derive(Debug)]
pub struct ConsumerOffsets {
    inner: HashMap<String, u64>,
    path: PathBuf,
}

impl ConsumerOffsets {
    pub fn new(inner: HashMap<String, u64>, path: PathBuf) -> Self {
        Self {
            inner,
            path,
        }
    }

    pub fn get(&self, customer_name: &str) -> Option<u64> {
        self.inner.get(customer_name).map(|v| *v)
    }

    pub fn insert(&mut self, consumer_name: &str, offset: u64) -> Result<(), Error> {
        self.inner.insert(consumer_name.to_owned(), offset);

        self.flush()
    }
    
    pub fn remove(&mut self, key: &str) {
        self.inner.remove(key);

        self.flush().ok();
    }

    pub fn set(&mut self, consumer_name: &str, offset: u64) {
        self.insert(consumer_name, offset).ok();

    }

    fn flush(&self) -> Result<(), Error> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)
            .unwrap();

        let data = serde_yaml::to_string(&self.inner).expect("serde_yaml::to_string() failed");

        file.write_all(data.as_bytes())
            .map_err(|error| Error::new_file(Some(error.into()), None))
    }

    pub fn new_from_file(path: PathBuf)->  Result<Self, Error> {
        let mut buf = String::new();

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .unwrap();
        file.read_to_string(&mut buf)
            .map_err(|error| Error::new_file(Some(error.into()), None))?;

        match serde_yaml::from_str::<HashMap<String, u64>>(&buf) {
            Ok(inner) => {
                return Ok(ConsumerOffsets::new(inner, path))
            },
            Err(error) => Err(Error::new_file(Some(error.into()), None)),
        }
    }
}