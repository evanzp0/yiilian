use std::path::PathBuf;

use memmap::MmapMut;

const CONSUMER_OFFSETS_FILE_NAME: &str = "__consumer_offsets";
const LOG_DATA__FILE_EXTENSION: &str = ".log";
const LOG_INDEX__FILE_EXTENSION: &str = ".index";
const TIME_INDEX__FILE_EXTENSION: &str = ".timeindex";

pub struct LogSegment {
    length: usize,
    max_length: usize,
    message_count: usize,
    offset: u64,
    prefix_path: PathBuf,
    log_data: MmapMut,
    log_index: MmapMut,
    time_index: MmapMut,
    consumer_offsets: MmapMut,
}