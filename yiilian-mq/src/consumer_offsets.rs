use std::{collections::HashMap, fs::File, io::{Read, Write}};

use yiilian_core::common::error::Error;

#[derive(Debug)]
pub struct ConsumerOffsets {
    inner: HashMap<String, u64>,
    file: File,
}

impl ConsumerOffsets {
    pub fn new(inner: HashMap<String, u64>, file: File) -> Self {
        Self {
            inner,
            file,
        }
    }

    pub fn insert(&mut self, k: &str, v: u64) -> Result<(), Error> {
        self.inner.insert(k.to_owned(), v);

        self.flush()
    }

    fn flush(&mut self) -> Result<(), Error> {
        let data = serde_yaml::to_string(&self.inner).expect("serde_yaml::to_string() failed");
        self.file.write_all(data.as_bytes())
            .map_err(|error| Error::new_file(Some(error.into()), None))
    }

    pub fn new_from_file(mut file: File)->  Result<Self, Error> {
        let mut buf = String::new();
        file.read_to_string(&mut buf)
            .map_err(|error| Error::new_file(Some(error.into()), None))?;

        match serde_yaml::from_str::<HashMap<String, u64>>(&buf) {
            Ok(inner) => {
                return Ok(ConsumerOffsets::new(inner, file))
            },
            Err(error) => Err(Error::new_file(Some(error.into()), None)),
        }
    }
}