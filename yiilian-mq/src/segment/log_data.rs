use std::{fs::File, io::Write};

use bytes::Bytes;
use memmap::MmapMut;
use yiilian_core::common::error::Error;

use crate::message::{Message, MESSAGE_LENGTH_LEN, MESSAGE_PREFIX_LEN, MIN_MESSAGE_LEN};

const DEFAULT_LOG_DATA_CAPACITY: usize = 10 * 1024 * 1024;

const DATA_PREFIX_LEN: usize = 8;

#[derive(Debug)]
/// LogData = length(8) + messages
pub struct LogData {
    length: usize,
    offset: u64,
    base_file: File,
    cache: MmapMut,
}

impl LogData {
    pub fn new(offset: u64, base_file: File, capacity: Option<usize>) -> Result<Self, Error> {
        let capacity = capacity.map_or(DEFAULT_LOG_DATA_CAPACITY, |val| {
            if val < MIN_MESSAGE_LEN {
                DEFAULT_LOG_DATA_CAPACITY
            } else {
                val
            }
        });

        base_file.set_len(capacity as u64).map_err(|error| {
            Error::new_memory(
                Some(error.into()),
                Some("Set file len for memory mapping is failed".to_owned()),
            )
        })?;

        let cache = unsafe {
            MmapMut::map_mut(&base_file).map_err(|error| {
                Error::new_memory(
                    Some(error.into()),
                    Some("Mapping memory from file is failed".to_owned()),
                )
            })?
        };

        let length =
            usize::from_be_bytes(cache[0..8].try_into().expect("Incorrect mem cache length"));

        Ok(LogData {
            length,
            offset,
            base_file,
            cache,
        })
    }

    pub fn capacity(&self) -> usize {
        self.cache.len()
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn base_file(&self) -> &File {
        &self.base_file
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn set_len(&mut self, length: usize) {
        self.length = length;
        let length: [u8; 8] = length.to_be_bytes();
        self.cache[0..8].copy_from_slice(&length);
    }
}


impl LogData {
    pub fn push(&mut self, message: Message) -> Result<(), Error> {
        let start_pos = DATA_PREFIX_LEN + self.length;
        let msg_len = message.len();

        if start_pos + msg_len > self.capacity() {
            Err(Error::new_general("push message over capacity limited"))?
        }

        let message_bytes: Bytes = message.into();

        (&mut self.cache[start_pos..]).write_all(&message_bytes).map_err(|error| {
            Error::new_memory(
                Some(error.into()),
                Some("writing Cache is failed".to_owned()),
            )
        })?;

        self.set_len(msg_len);

        Ok(())
    }

    pub fn get_message_by_pos(&self, start_pos: usize) -> Option<Message> {
        if start_pos + MESSAGE_PREFIX_LEN > self.length {
            return None
        }

        let message_len = {
            let val = &self.cache[start_pos..start_pos + MESSAGE_LENGTH_LEN];
            let val = usize::from_be_bytes(val.try_into().expect("message length bytes is invalid"));

            val
        };

        let end_pos = start_pos + MESSAGE_PREFIX_LEN + message_len;
        if end_pos > self.length {
            return None
        }

        let message = &self.cache[start_pos..end_pos];

        match message.try_into() {
            Ok(val) => Some(val),
            Err(_) => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::OpenOptions, path::PathBuf};

    use bytes::Bytes;
    use chrono::Utc;

    use crate::message::Message;

    use super::LogData;


    #[test]
    fn test_get_message_by_pos() {
        let offset = 1;
        let capacity = Some(100);
        let path: PathBuf = "./test_file.txt".into();
        let base_file = OpenOptions::new()
                               .read(true)
                               .write(true)
                               .create(true)
                               .open(&path)
                               .unwrap();

        let mut log_data = LogData::new(offset, base_file, capacity).unwrap();

        let value: Bytes = b"12"[..].into();
        let timestamp: i64 = Utc::now().timestamp_millis();
        let message = Message::new(offset, timestamp, value);

        log_data.push(message).unwrap();

        println!("{:?}", log_data);
        println!("{:?}", &log_data.cache[..]);
    }
}