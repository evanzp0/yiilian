use std::{fs::OpenOptions, path::PathBuf};

use memmap::MmapMut;
use yiilian_core::common::error::Error;

use crate::{message::Message, segment::{
    calc_log_index_size, LOG_DATA_FILE_EXTENSION, LOG_DATA_SIZE, LOG_INDEX_FILE_EXTENSION,
}};

use super::{log_data::LogData, log_index::{LogIndex, LogIndexItem}};

pub struct ActiveSegment {
    offset: u64,
    base_path: PathBuf,
    log_data: LogData,
    log_index: LogIndex,
}

impl ActiveSegment {
    pub fn new(offset: u64, base_path: PathBuf) -> Result<Self, Error> {
        let log_data_file = {
            let mut path = base_path.clone();
            path.push(format!("{:0>20}.{}", offset, LOG_DATA_FILE_EXTENSION));
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&path)
                .map_err(|error| Error::new_file(Some(error.into()), None))?;
            file.set_len(LOG_DATA_SIZE as u64)
                .map_err(|error| Error::new_file(Some(error.into()), None))?;
            file
        };

        let log_index_file = {
            let mut path = base_path.clone();
            path.push(format!("{:0>20}.{}", offset, LOG_INDEX_FILE_EXTENSION));
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&path)
                .map_err(|error| Error::new_file(Some(error.into()), None))?;
            file.set_len(calc_log_index_size(LOG_DATA_SIZE) as u64)
                .map_err(|error| Error::new_file(Some(error.into()), None))?;
            file
        };

        let cache = unsafe {
            MmapMut::map_mut(&log_data_file)
                .map_err(|error| Error::new_memory(Some(error.into()), None))?
        };
        let log_data = LogData::new(offset, cache)?;

        let cache = unsafe {
            MmapMut::map_mut(&log_index_file)
                .map_err(|error| Error::new_memory(Some(error.into()), None))?
        };
        let log_index = LogIndex::new(offset, cache)?;

        Ok(ActiveSegment {
            offset,
            base_path,
            log_data,
            log_index,
        })
    }

    pub fn push_message(&mut self, message: Message) -> Result<(), Error> {
        let index_item = LogIndexItem::new(message.offset(), self.log_data.len());
        self.log_data.push(message)?;
        self.log_index.push(index_item)?;

        Ok(())
    }
}