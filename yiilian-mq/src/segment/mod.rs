use std::{fs::OpenOptions, path::PathBuf};

use yiilian_core::common::error::Error;

use crate::{message::Message, segment::{log_data::log_data_file::LogDataFile, log_index::log_index_file::LogIndexFile}};

pub mod active_segment;
pub mod log_data;
pub mod log_index;

pub const CONSUMER_OFFSETS_FILE_NAME: &str = "__consumer_offsets";
pub const LOG_DATA_FILE_EXTENSION: &str = "log";
pub const LOG_INDEX_FILE_EXTENSION: &str = "index";
pub const TIME_INDEX_FILE_EXTENSION: &str = "timeindex";

// const  LOG_DATA_SIZE: usize = 10 * 1024 * 1024;
const LOG_DATA_SIZE: usize = 100;

pub struct Segment {
    // length: usize,
    // max_length: usize,
    // message_count: usize,
    // offset: u64,
    // prefix_path: PathBuf,
    // log_data: MmapMut,
    // log_index: MmapMut,
    // time_index: MmapMut,
    // consumer_offsets: MmapMut,
}

pub fn calc_log_index_size(log_data_size: usize) -> usize {
    let mut msg_cnt = (log_data_size - log_data::LOGDATA_PREFIX_LEN) / 24;
    let y = (log_data_size - log_data::LOGDATA_PREFIX_LEN) % 24;
    if y > 0 {
        msg_cnt += 1;
    }

    let index_size = msg_cnt * log_index::LOGINDEX_ITEM_LEN + log_index::LOGINDEX_PREFIX_LEN;

    index_size
}

pub fn poll_message(
    topic_path: &PathBuf,
    segment_offset: u64,
    target_offset: u64,
) -> Result<Option<Message>, Error> {
    let index_file_name = gen_mq_file_name(segment_offset, LOG_INDEX_FILE_EXTENSION);
    let data_file_name = gen_mq_file_name(segment_offset, LOG_DATA_FILE_EXTENSION);

    let mut index_path = topic_path.to_owned();
    index_path.push(index_file_name);
    let index_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&index_path)
        .map_err(|error| Error::new_file(Some(error.into()), None))?;

    let mut log_index_file = LogIndexFile::new(segment_offset, index_file)?;
    let index_item = log_index_file.get_by_offset(target_offset);

    let mut data_path = topic_path.to_owned();
    data_path.push(data_file_name);
    let data_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&data_path)
        .map_err(|error| Error::new_file(Some(error.into()), None))?;


    let mut log_data_file = LogDataFile::new(segment_offset, data_file)?;

    if let Some(index_item) = index_item {
        let message = log_data_file.get_message(target_offset, index_item.message_pos());

        Ok(message)
    } else {
        Ok(None)
    }
}

pub fn gen_mq_file_name(segment_offset: u64, ext: &str) -> String {
    format!("{:0>20}.{}", segment_offset, ext)
}
