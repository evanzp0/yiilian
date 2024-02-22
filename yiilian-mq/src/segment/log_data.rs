use std::io::Write;

use bytes::Bytes;
use memmap::MmapMut;
use yiilian_core::common::error::Error;

use crate::message::{Message, MESSAGE_PREFIX_LEN};

const LOGDATA_PREFIX_LEN: usize = 8;

#[derive(Debug)]
/// LogData = length(8) + [ message .. ]
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
            usize::from_be_bytes(cache[0..8].try_into().expect("Incorrect mem cache length for LogData"))
        };

        Ok(LogData {
            length,
            offset,
            cache,
        })
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

    pub fn clear(&mut self) {
        self.cache.fill(0);
        self.length = 0;
    }

    /// 返回 LogData 末尾的 pos, 不包含 LOGDATA_PREFIX_LEN
    pub fn push(&mut self, message: Message) -> Result<usize, Error> {
        let start_pos = LOGDATA_PREFIX_LEN + self.len();
        let msg_total_size = message.total_size();

        if message.total_size() > self.free_space() {
            Err(Error::new_general("Push message for LogData over capacity limited"))?
        }

        let message_bytes: Bytes = message.into();

        (&mut self.cache[start_pos..])
            .write_all(&message_bytes)
            .map_err(|error| {
                Error::new_memory(
                    Some(error.into()),
                    Some("writing Cache for LogData is failed".to_owned()),
                )
            })?;

        self.set_len(self.len() + msg_total_size);

        Ok(self.len())
    }

    /// start_pos 不包含 LOGDATA_PREFIX_LEN
    // 返回找到的消息，以及下一次起始位置
    pub fn next(&self, pos: usize) -> Option<(Message, usize)> {
        let cache = &self.cache[LOGDATA_PREFIX_LEN..];

        if pos + MESSAGE_PREFIX_LEN > self.len() {
            return None;
        }

        let message_len = {
            let val = &cache[pos..pos + MESSAGE_PREFIX_LEN];
            let val =
                u32::from_be_bytes(val.try_into().expect("Message length bytes is invalid"));

            val
        } as usize;

        let end_pos = pos + MESSAGE_PREFIX_LEN + message_len;
        if end_pos > self.len() {
            return None;
        }

        let message = &cache[pos..end_pos];

        match message.try_into() {
            Ok(val) => Some((val, end_pos)),
            Err(_) => return None,
        }
    }

    /// 从指定位置开始查找 offset 的消息
    pub fn get_message(&self, offset: u64, mut pos: usize) -> Option<Message> {
        while pos < self.len() {

            if let Some((message, inner_pos)) = self.next(pos) {
                
                if message.offset() == offset {
                    return Some(message)
                } else if message.offset() > offset {
                    return None
                }

                pos = inner_pos;
            } else {
                break;
            }
        }

        None
    }

    pub fn get_messages(&self, mut pos: usize, expected_count: usize) -> Option<Vec<Message>> {
        let mut count = 1;
        let mut messages = vec![];
        
        while pos < self.len() && count <= expected_count {

            if let Some((message, inner_pos)) = self.next(pos) {
                messages.push(message);
                pos = inner_pos;
            } else {
                break;
            }

            count += 1;
        }

        if messages.len() > 0 {
            Some(messages)
        } else {
            None
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
    fn test_get_message() {
        let offset = 1;
        let cache = MmapMut::map_anon(100).unwrap();
        let mut log_data = LogData::new(offset, cache).unwrap();

        let value: Bytes = b"11"[..].into();
        let timestamp: i64 = Utc::now().timestamp_millis();
        let message = Message::new(offset, timestamp, value);

        log_data.push(message).unwrap();

        let offset = 2;
        let value: Bytes = b"12"[..].into();
        let timestamp: i64 = Utc::now().timestamp_millis();
        let message = Message::new(offset, timestamp, value);

        log_data.push(message).unwrap();

        let offset = 3;
        let value: Bytes = b"13"[..].into();
        let timestamp: i64 = Utc::now().timestamp_millis();
        let message = Message::new(offset, timestamp, value);

        log_data.push(message).unwrap();

        let cache_len =
            usize::from_be_bytes(log_data.cache[0..LOGDATA_PREFIX_LEN].try_into().unwrap());
        assert_eq!(cache_len, log_data.len());

        let (message, pos) = log_data.next(26).unwrap();
        assert_eq!(2, message.offset());

        let (message, pos) = log_data.next(pos).unwrap();
        assert_eq!(3, message.offset());

        assert_eq!(pos, log_data.len());

        let message = log_data.next(27);
        assert_eq!(true, message.is_none());

        let messages = log_data.get_messages(26, 3).unwrap();
        assert_eq!(2, messages.len());

        let messages = log_data.get_messages(100, 3);
        assert_eq!(true, messages.is_none());

        let message = log_data.get_message(3, 26).unwrap();
        assert_eq!(3, message.offset());

        let message = log_data.get_message(4, 26);
        assert_eq!(true, message.is_none());
    }
}
