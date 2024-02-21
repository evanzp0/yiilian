use std::io::Write;

use bytes::Bytes;
use memmap::MmapMut;
use yiilian_core::common::error::Error;

use crate::message::{Message, MESSAGE_PREFIX_LEN};

const LOGDATA_PREFIX_LEN: usize = 8;

#[derive(Debug)]
/// LogData = length(8) + messages
pub struct LogData {
    length: usize,
    offset: u64,
    cache: MmapMut,
}

impl LogData {
    pub fn new(offset: u64, cache: MmapMut) -> Result<Self, Error> {
        let length = if cache.len() < LOGDATA_PREFIX_LEN {
            Err(Error::new_memory(None, Some(format!("cache size can't less than {LOGDATA_PREFIX_LEN} bytes"))))?
        } else {
            usize::from_be_bytes(cache[0..8].try_into().expect("Incorrect mem cache length"))
        };

        Ok(LogData {
            length,
            offset,
            cache,
        })
    }

    pub fn clear(&mut self) {
        self.cache.fill(0);
        self.length = 0;
    }

    pub fn capacity(&self) -> usize {
        self.cache.len()
    }

    pub fn total_size(&self) -> usize {
        self.length + LOGDATA_PREFIX_LEN
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn free_space(&self) -> usize {
        self.capacity() - self.total_size()
    }

    pub fn set_len(&mut self, length: usize) {
        self.length = length;
        let length: [u8; 8] = length.to_be_bytes();
        self.cache[0..8].copy_from_slice(&length);
    }
}

impl LogData {
    pub fn push(&mut self, message: Message) -> Result<(), Error> {
        let start_pos = LOGDATA_PREFIX_LEN + self.length;
        let msg_total_size = message.total_size();

        if start_pos + message.total_size() > self.capacity() {
            Err(Error::new_general("Push message over capacity limited"))?
        }

        let message_bytes: Bytes = message.into();

        (&mut self.cache[start_pos..])
            .write_all(&message_bytes)
            .map_err(|error| {
                Error::new_memory(
                    Some(error.into()),
                    Some("writing Cache is failed".to_owned()),
                )
            })?;

        self.set_len(self.length + msg_total_size);

        Ok(())
    }

    pub fn get_message_by_pos(&self, start_pos: usize) -> Option<Message> {
        if start_pos + MESSAGE_PREFIX_LEN > self.length {
            return None;
        }

        let message_len = {
            let val = &self.cache[start_pos..start_pos + MESSAGE_PREFIX_LEN];
            let val =
                usize::from_be_bytes(val.try_into().expect("message length bytes is invalid"));

            val
        };

        let end_pos = start_pos + MESSAGE_PREFIX_LEN + message_len;
        if end_pos > self.length {
            return None;
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

    use bytes::Bytes;
    use chrono::Utc;
    use memmap::MmapMut;

    use crate::{message::Message, segment::log_data::LOGDATA_PREFIX_LEN};

    use super::LogData;

    #[test]
    fn test_get_message_by_pos() {
        let offset = 1;
        let cache = MmapMut::map_anon(100).unwrap();

        let mut log_data = LogData::new(offset, cache).unwrap();

        let value: Bytes = b"12"[..].into();
        let timestamp: i64 = Utc::now().timestamp_millis();
        let message = Message::new(offset, timestamp, value);

        log_data.push(message).unwrap();

        let cache_len =
            usize::from_be_bytes(log_data.cache[0..LOGDATA_PREFIX_LEN].try_into().unwrap());
        assert_eq!(cache_len, log_data.len());
    }
}
