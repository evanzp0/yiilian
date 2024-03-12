
use std::{fs::File, io::{Read, Seek, SeekFrom}};

use yiilian_core::common::error::Error;

use crate::message::{Message, MESSAGE_PREFIX_LEN};

const LOGDATA_PREFIX_LEN: usize = 8;

#[derive(Debug)]
/// LogData = length(8) + [ message .. ]
pub struct LogDataFile {
    length: usize,
    offset: u64,
    file: File,
}

impl LogDataFile {
    pub fn new(offset: u64, mut file: File) -> Result<Self, Error> {
        let capacity = file.metadata().expect("Get file metadata error").len() as usize;
        let length = if capacity < LOGDATA_PREFIX_LEN {
            Err(Error::new_file(None, Some(format!("File size can't less than {LOGDATA_PREFIX_LEN} bytes"))))?
        } else {
            file.seek(SeekFrom::Start(0)).expect("seek error");
            let mut buf = [0; 8];
            file.read_exact(&mut buf).expect("read_exact error");

            usize::from_be_bytes(buf)
        };

        Ok(LogDataFile {
            length,
            offset,
            file,
        })
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
}

impl LogDataFile {

    pub fn reset(&mut self) {
        self.file.seek(SeekFrom::Start(LOGDATA_PREFIX_LEN as u64)).expect("reset error");
    }

    /// start_pos 不包含 LOGDATA_PREFIX_LEN
    /// 返回找到的消息，以及下一次起始位置
    pub fn next(&mut self, pos: usize) -> Option<(Message, usize)> {

        if pos + MESSAGE_PREFIX_LEN > self.len() {
            return None;
        }
        
        let message_len = {
            let mut buf = [0; MESSAGE_PREFIX_LEN];
            self.file.seek(SeekFrom::Start((pos + LOGDATA_PREFIX_LEN) as u64)).expect("seek error");
            self.file.read_exact(&mut buf).expect("read_exact error");

            let val = u32::from_be_bytes(buf);

            val
        } as usize;

        let end_pos = pos + MESSAGE_PREFIX_LEN + message_len;
        if end_pos > self.len() {
            return None;
        }

        let message = {
            self.file.seek(SeekFrom::Start((pos + LOGDATA_PREFIX_LEN) as u64)).expect("seek error");
            let mut buf = vec![0;  MESSAGE_PREFIX_LEN + message_len];
            self.file.read_exact(&mut buf).expect("read_exact error");

            buf
        };

        match message.as_slice().try_into() {
            Ok(val) => Some((val, end_pos)),
            Err(_) => return None,
        }
    }

    /// 从指定位置开始查找 offset 的消息
    pub fn get_message(&mut self, offset: u64, mut pos: usize) -> Option<Message> {
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

    pub fn get_messages(&mut self, mut pos: usize, expected_count: usize) -> Option<Vec<Message>> {
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
    use std::{fs::OpenOptions, io::Write, path::PathBuf};

    use bytes::{BufMut, Bytes, BytesMut};
    use chrono::Utc;

    use crate::message::Message;

    use super::LogDataFile;

    #[test]
    fn test_get_message() {
        let path: PathBuf = "./test_log_data_file.txt".into();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .unwrap();

        file.set_len(100).unwrap();

        let mut buf = BytesMut::new();
        buf.put_u64(78);

        let offset = 1;
        let value: Bytes = b"11"[..].into();
        let timestamp: i64 = Utc::now().timestamp_millis();
        let message: Bytes = Message::new(offset, timestamp, value).into();
        buf.put_slice(&message);

        let offset = 2;
        let value: Bytes = b"12"[..].into();
        let timestamp: i64 = Utc::now().timestamp_millis();
        let message: Bytes = Message::new(offset, timestamp, value).into();
        buf.put_slice(&message);

        let offset = 3;
        let value: Bytes = b"13"[..].into();
        let timestamp: i64 = Utc::now().timestamp_millis();
        let message: Bytes = Message::new(offset, timestamp, value).into();
        buf.put_slice(&message);

        file.write(&buf).unwrap();

        let mut log_data_file = LogDataFile::new(0, file).unwrap();

        let messages = log_data_file.get_messages(0, 2).unwrap();
        assert_eq!(2, messages.len());

        let message = log_data_file.get_message(3, 26).unwrap();
        assert_eq!(3, message.offset());

        let message = log_data_file.get_message(4, 26);
        assert_eq!(true, message.is_none());

        std::fs::remove_file(path).unwrap();
    }
}
