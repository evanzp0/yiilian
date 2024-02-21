use std::fs::File;

use memmap::MmapMut;
use yiilian_core::common::error::Error;

use crate::message::MIN_MESSAGE_LEN;

const DEFAULT_LOG_DATA_CAPACITY: usize = 10 * 1024 * 1024;

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
            Error::new_allocate(
                Some(error.into()),
                Some("Set file len for memory mapping is failed".to_owned()),
            )
        })?;

        let cache = unsafe {
            MmapMut::map_mut(&base_file).map_err(|error| {
                Error::new_allocate(
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
}
