pub mod log_data;
pub mod log_index;
pub mod active_segment;


pub const CONSUMER_OFFSETS_FILE_NAME: &str = "__consumer_offsets";
pub const LOG_DATA_FILE_EXTENSION: &str = "log";
pub const LOG_INDEX_FILE_EXTENSION: &str = "index";
pub const TIME_INDEX_FILE_EXTENSION: &str = "timeindex";

const  LOG_DATA_SIZE: usize = 10 * 1024 * 1024;

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