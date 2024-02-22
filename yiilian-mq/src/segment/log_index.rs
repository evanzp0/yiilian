use std::io::Write;

use bytes::{BufMut, Bytes, BytesMut};
use memmap::MmapMut;
use yiilian_core::common::error::Error;

const LOGINDEX_PREFIX_LEN: usize = 8;
const LOGINDEX_ITEM_LEN: usize = 16;

/// LogIndex = len(8) + [ message_offset(8) + message_pos(8) .. ]
pub struct LogIndex {
    length: usize,
    offset: u64,
    cache: MmapMut,
}

impl LogIndex {

    pub fn new(offset: u64, cache: MmapMut) -> Result<Self, Error> {
        let length = if cache.len() < LOGINDEX_PREFIX_LEN {
            Err(Error::new_memory(None, Some(format!("cache size can't less than {LOGINDEX_PREFIX_LEN} bytes"))))?
        } else {
            usize::from_be_bytes(cache[0..8].try_into().expect("Incorrect mem cache length for LogIndex"))
        };

        Ok(LogIndex {
            length,
            offset,
            cache,
        })
    }

    pub fn capacity(&self) -> usize {
        self.cache.len()
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

    pub fn set_len(&mut self, length: usize) {
        self.length = length;
        let length: [u8; 8] = length.to_be_bytes();
        self.cache[0..8].copy_from_slice(&length);
    }


}

impl LogIndex {
    
    pub fn count(&self) -> usize {
        self.len() / LOGINDEX_ITEM_LEN
    }

    pub fn get(&self, index: usize) -> Option<LogIndexItem> {

        if index >= self.count() {
            return None
        }

        let cache = &self.cache[LOGINDEX_PREFIX_LEN..];
        let start = index * LOGINDEX_ITEM_LEN;
        let item_bytes = &cache[start..start + LOGINDEX_ITEM_LEN];

        item_bytes
            .try_into()
            .map_or(None, |val| Some(val))
    }

    pub fn get_by_offset(&self, target_offset: u64) -> Option<LogIndexItem> {
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

    pub fn clear(&mut self) {
        self.cache.fill(0);
        self.length = 0;
    }

    pub fn push(&mut self, item: LogIndexItem) -> Result<usize, Error> {
        let start_pos = LOGINDEX_PREFIX_LEN + self.len();

        if LOGINDEX_ITEM_LEN > self.free_space() {
            Err(Error::new_general("Push message for LogIndex over capacity limited"))?
        }

        let index_item: Bytes = item.into();

        (&mut self.cache[start_pos..])
            .write_all(&index_item)
            .map_err(|error| {
                Error::new_memory(
                    Some(error.into()),
                    Some("writing Cache for LogIndex is failed".to_owned()),
                )
            })?;

        self.set_len(self.length + LOGINDEX_ITEM_LEN);

        Ok(self.len())
    }

}

#[derive(Debug)]
pub struct LogIndexItem {
    message_offset: u64, 
    message_pos: usize,
}

impl LogIndexItem {
    pub fn new(message_offset: u64, message_pos: usize) -> Self {
        LogIndexItem {
            message_offset,
            message_pos,
        }
    }

    fn message_offset(&self) -> u64 {
        self.message_offset
    }


    fn message_pos(&self) -> usize {
        self.message_pos
    }
}

impl From<LogIndexItem> for Bytes {
    fn from(value: LogIndexItem) -> Self {
        let mut rst = BytesMut::new();
        rst.put_u64(value.message_offset);
        rst.put_u64(value.message_pos as u64);

        rst.into()
    }
}

impl TryFrom<&[u8]> for LogIndexItem {
    type Error = Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() < LOGINDEX_ITEM_LEN {
            Err(Error::new_decode(&format!("Data is too short to decode LogIndexItem: {:?}", data.len())))?;
        }

        let message_offset = u64::from_be_bytes(data[0..8].try_into().expect("data[0..8] is not satisfy"));
        let message_pos = usize::from_be_bytes(data[8..16].try_into().expect("data[0..8] is not satisfy"));

        Ok(LogIndexItem::new(message_offset, message_pos))
    }
}

#[cfg(test)]
mod tests {
    use memmap::MmapMut;

    use super::{LogIndex, LogIndexItem};

    #[test]
    fn test_log_index() {
        let offset = 1;
        let cache = MmapMut::map_anon(60).unwrap();
        let mut log_index = LogIndex::new(offset, cache).unwrap();

        let item = LogIndexItem::new(0, 101);
        log_index.push(item).unwrap();

        let item = LogIndexItem::new(1, 102);
        log_index.push(item).unwrap();

        let item = LogIndexItem::new(2, 103);
        log_index.push(item).unwrap();

        assert_eq!(56, log_index.total_size());

        let item = LogIndexItem::new(3, 104);
        let rst = log_index.push(item);
        assert_eq!(true, rst.is_err());

        let item = log_index.get(2).unwrap();
        assert_eq!(2, item.message_offset);

        let item = log_index.get_by_offset(2).unwrap();
        assert_eq!(2, item.message_offset());

        let rst = log_index.get_by_offset(3);
        assert_eq!(true, rst.is_none());
    }
}