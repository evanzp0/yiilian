
use std::{fs::File, io::{Read, Seek, SeekFrom}};

use yiilian_core::common::error::Error;

use super::{LogIndexItem, LOGINDEX_ITEM_LEN, LOGINDEX_PREFIX_LEN};

/// LogIndex = len(8) + [ message_offset(8) + message_pos(8) .. ]
pub struct LogIndexFile {
    length: usize,
    offset: u64,
    file: File,
    capacity: usize,
}

impl LogIndexFile {

    pub fn new(offset: u64, mut file: File) -> Result<Self, Error> {
        let capacity = file.metadata().expect("Get file metadata error").len() as usize;
        let length = if capacity < LOGINDEX_PREFIX_LEN {
            Err(Error::new_memory(None, Some(format!("file size can't less than {LOGINDEX_PREFIX_LEN} bytes"))))?
        } else {
            file.seek(SeekFrom::Start(0)).expect("seek error");
            let mut buf = [0; 8];
            file.read_exact(&mut buf).expect("read_exact error");

            usize::from_be_bytes(buf)
        };

        Ok(LogIndexFile {
            length,
            offset,
            file,
            capacity,
        })
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn total_size(&self) -> usize {
        self.length + LOGINDEX_PREFIX_LEN
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
}

impl LogIndexFile {

    pub fn reset(&mut self) {
        self.file.seek(SeekFrom::Start(LOGINDEX_PREFIX_LEN as u64)).expect("reset error");
    }
    
    pub fn count(&self) -> usize {
        self.len() / LOGINDEX_ITEM_LEN
    }

    pub fn get(&mut self, index: usize) -> Option<LogIndexItem> {

        if index >= self.count() {
            return None
        }

        self.reset();

        let start = index * LOGINDEX_ITEM_LEN;
        self.file.seek(SeekFrom::Current(start as i64)).expect("seek error");
        let mut item_bytes = [0; LOGINDEX_ITEM_LEN];
        self.file.read_exact(&mut item_bytes).expect("read_exact error");
        
        item_bytes
            .as_slice()
            .try_into()
            .map_or(None, |val| Some(val))
    }

    pub fn get_by_offset(&mut self, target_offset: u64) -> Option<LogIndexItem> {
        let mut left = 0;
        let mut right = self.count() - 1;

        while left <= right {
            let mid = left + (right - left) / 2;
            let mid_item = self.get(mid).expect("Not found mid item in LogIndex");

            if mid_item.message_offset() == target_offset {
                return Some(mid_item)
            } else if mid_item.message_offset() < target_offset {
                left = mid + 1;
            } else if mid_item.message_offset() > target_offset {
                right = mid - 1;
            }
        }

        None
    }
}


#[cfg(test)]
mod tests {
    use std::{fs::{self, OpenOptions}, io::Write, path::PathBuf};

    use bytes::{BufMut, Bytes, BytesMut};

    use crate::segment::log_index::LogIndexItem;

    use super::LogIndexFile;


    #[test]
    fn test_log_index() {
        let path: PathBuf = "./test_log_index_file.txt".into();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .unwrap();
        file.set_len(56).unwrap();

        let mut buf = BytesMut::new();
        buf.put_u64(56);

        let item: Bytes = LogIndexItem::new(0, 101).into();
        buf.put_slice(&item);

        let item: Bytes = LogIndexItem::new(1, 102).into();
        buf.put_slice(&item);

        let item: Bytes = LogIndexItem::new(2, 103).into();
        buf.put_slice(&item);

        file.write(&buf).unwrap();

        let mut log_index_file = LogIndexFile::new(0, file).unwrap();

        let item = log_index_file.get(2).unwrap();
        assert_eq!(2, item.message_offset());

        let item = log_index_file.get_by_offset(1).unwrap();
        assert_eq!(1, item.message_offset());

        fs::remove_file(path).unwrap();
    }
}