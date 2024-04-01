use std::{
    fs::{self, OpenOptions},
    path::PathBuf,
    time::{Duration, SystemTime},
};

use chrono::Utc;
use yiilian_core::common::{
    error::Error,
    util::{atoi, binary_insert},
};

use crate::{
    consumer_offsets::ConsumerOffsets,
    message::{in_message::InMessage, Message, MESSAGE_PREFIX_LEN},
    segment::{
        active_segment::ActiveSegment, gen_mq_file_name, log_index::log_index_file::LogIndexFile,
        poll_message_inner, LOG_DATA_FILE_EXTENSION, LOG_INDEX_FILE_EXTENSION,
    },
};

const KEEP_SEGMENT_SECS: u64 = 24 * 60 * 60 * 3;

#[derive(Debug)]
pub struct Topic {
    #[allow(unused)]
    name: String,
    log_data_size: usize,
    path: PathBuf,
    active_segment: ActiveSegment,
    consumers: ConsumerOffsets,
    segment_offsets: Vec<SegmentInfo>,
}

impl Topic {
    pub fn new(name: &str, path: PathBuf, log_data_size: usize) -> Result<Self, Error> {
        let entries = fs::read_dir(path.clone())
            .map_err(|error| Error::new_file(Some(error.into()), None))?;

        let mut segment_offsets: Vec<SegmentInfo> = vec![];
        for entry in entries {
            let entry = entry.map_err(|error| Error::new_file(Some(error.into()), None))?;

            let path = entry.path();

            let metadata =
                fs::metadata(&path).map_err(|error| Error::new_file(Some(error.into()), None))?;

            if metadata.is_file() {
                if let Some(file_name) = path.file_name() {
                    if let Some(file_name) = file_name.to_str() {
                        if file_name.ends_with(".log") {
                            let offset_len = file_name.len() - 4;

                            let offset: u64 = atoi(file_name[0..offset_len].as_bytes())?;
                            let mod_time: SystemTime = metadata.modified().expect("mod time error");

                            let segment_info = SegmentInfo::new(offset, mod_time);

                            binary_insert(&mut segment_offsets, segment_info, false);
                        }
                    }
                }
            }
        }

        if segment_offsets.len() == 0 {
            segment_offsets.push(SegmentInfo::new(0, SystemTime::now()));
        }

        let last_segment_offset = segment_offsets
            .get(segment_offsets.len() - 1)
            .expect("segment_offsets should exist")
            .offset;
        let active_segment = ActiveSegment::new(last_segment_offset, path.clone(), log_data_size)?;

        let consumer_offsets_path: PathBuf = {
            let mut p = path.clone();
            p.push("_consumer_offsets");
            p
        };

        let consumer_offsets = ConsumerOffsets::new_from_file(consumer_offsets_path)?;

        Ok(Topic {
            name: name.to_owned(),
            path,
            active_segment,
            consumers: consumer_offsets,
            segment_offsets,
            log_data_size,
        })
    }

    pub fn consumer_offsets(&mut self) -> &mut ConsumerOffsets {
        return &mut self.consumers;
    }

    pub fn remove_consumer(&mut self, consumer_name: &str) {
        self.consumers.remove(consumer_name)
    }

    pub fn segment_offsets(&self) -> &Vec<SegmentInfo> {
        &self.segment_offsets
    }

    pub fn push_message(&mut self, message: InMessage) -> Result<(), Error> {
        let message_size = 20 + message.0.len() + MESSAGE_PREFIX_LEN;

        let enough_space = self.active_segment.enough_space(message_size);

        if !enough_space {
            let new_offset = self.active_segment.get_next_offset();
            let active_segment =
                ActiveSegment::new(new_offset, self.path.clone(), self.log_data_size)?;
            let segment_info = SegmentInfo::new(new_offset, SystemTime::now());
            self.segment_offsets.push(segment_info);
            self.active_segment = active_segment;
        }

        let new_offset = self.active_segment.get_next_offset();
        let message = Message::new(new_offset, Utc::now().timestamp_millis(), message.0);

        self.active_segment.push_message(message)
    }

    pub fn count(&self, customer_name: &str) -> u64 {
        let mut count = 0;

        if let Some(consumer_offset) = self.consumers.get(customer_name) {
            if let Some(segment_offset) = get_floor_offset(consumer_offset, &self.segment_offsets) {
                let index_file_name = gen_mq_file_name(segment_offset, LOG_INDEX_FILE_EXTENSION);
                let mut index_path = self.path.to_owned();

                index_path.push(index_file_name);
                let index_file = match OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(&index_path)
                {
                    Ok(file) => file,
                    Err(error) => {
                        println!("{}", error);
                        log::trace!(target: "yiilian-mq::topic", "{}", error);
                        return 0
                    },
                };

                let mut log_index_file = match LogIndexFile::new(segment_offset, index_file)
                {
                    Ok(file) => file,
                    Err(error) => {
                        println!("{}", error);
                        log::trace!(target: "yiilian-mq::topic", "{}", error);
                        return 0
                    },
                };

                let current_gap = if let Some(last_index_item) = log_index_file.last() {
                    let last_offset = last_index_item.message_offset();
                    last_offset - consumer_offset
                } else {
                    0
                };

                for segment_info in &self.segment_offsets {
                    if segment_info.offset <= segment_offset {
                        continue;
                    }

                    let index_file_name =
                        gen_mq_file_name(segment_offset, LOG_INDEX_FILE_EXTENSION);
                    let mut index_path = self.path.to_owned();

                    index_path.push(index_file_name);
                    let index_file = match OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .open(&index_path)
                        .map_err(|error| Error::new_file(Some(error.into()), None))
                        {
                            Ok(file) => file,
                            Err(error) => {
                                println!("{}", error);
                                log::trace!(target: "yiilian-mq::topic", "{}", error);
                                return 0
                            },
                        };

                    let log_index_file = match LogIndexFile::new(segment_offset, index_file) 
                    {
                        Ok(file) => file,
                        Err(error) => {
                            println!("{}", error);
                            log::trace!(target: "yiilian-mq::topic", "{}", error);
                            return 0
                        },
                    };
                    count += log_index_file.count();
                }

                count += current_gap;
            }
        }

        count
    }

    pub fn poll_message(&mut self, customer_name: &str) -> Option<Message> {
        let (segment_offset, target_offset) =
            if let Some(mut target_offset) = self.consumers.get(customer_name) {
                target_offset += 1;

                // println!("{:?}", (self.get_segment_offset(target_offset), target_offset));

                if let Some(segment_offset) = self.get_segment_offset(target_offset) {
                    (segment_offset, target_offset)
                } else {
                    if let Some(segment_offset) = get_oldest_offset(&self.segment_offsets) {
                        (segment_offset, segment_offset)
                    } else {
                        return None;
                    }
                }
            } else {
                if let Some(segment_offset) = get_oldest_offset(&self.segment_offsets) {
                    (segment_offset, segment_offset)
                } else {
                    return None;
                }
            };

        if let Ok(message) = poll_message_inner(&self.path, segment_offset, target_offset) {
            if message.is_some() {
                self.consumers.insert(customer_name, target_offset).ok();
            }

            message
        } else {
            None
        }
    }

    pub fn get_segment_offset(&self, target_offset: u64) -> Option<u64> {
        get_floor_offset(target_offset, &self.segment_offsets)
    }

    /// 删除过期文件
    pub fn purge_segment(&mut self) {
        let outdate_segments =
            find_outdate_segment(&self.segment_offsets, self.active_segment.offset());

        self.segment_offsets
            .retain(|item| !outdate_segments.contains(&item.offset));

        for offset in outdate_segments {
            let data_file_path = {
                let file_name = gen_mq_file_name(offset, LOG_DATA_FILE_EXTENSION);
                let mut base_path = self.path.clone();
                base_path.push(file_name);
                base_path
            };

            let index_file_path = {
                let file_name = gen_mq_file_name(offset, LOG_INDEX_FILE_EXTENSION);
                let mut base_path = self.path.clone();
                base_path.push(file_name);
                base_path
            };

            fs::remove_file(data_file_path).ok();
            fs::remove_file(index_file_path).ok();
        }
    }
}

fn get_oldest_offset(array: &Vec<SegmentInfo>) -> Option<u64> {
    if array.len() == 0 {
        return None;
    }

    let mut oldest = array.first().expect("get oldest");

    for item in array {
        if item.mod_time < oldest.mod_time {
            oldest = item;
        }
    }

    Some(oldest.offset)
}

fn get_floor_offset(target_offset: u64, array: &Vec<SegmentInfo>) -> Option<u64> {
    if array.len() == 0 {
        return None;
    }

    let mut left: i32 = 0;
    let mut right: i32 = (array.len() - 1) as i32;
    let mut mid_offset;

    while left <= right {
        let mid = left + (right - left) / 2;
        mid_offset = array
            .get(mid as usize)
            .expect("Not found mid item in LogIndex")
            .offset;

        if mid_offset == target_offset {
            return Some(mid_offset);
        } else if mid_offset < target_offset {
            left = mid + 1;
        } else if mid_offset > target_offset {
            right = mid - 1;
        }
    }

    let left = if left > 0 { left - 1 } else { return None };

    mid_offset = array
        .get(left as usize)
        .expect("Not found mid item in LogIndex")
        .offset;

    Some(mid_offset)
}

fn find_outdate_segment(segment_infos: &Vec<SegmentInfo>, active_segment_offset: u64) -> Vec<u64> {
    let now = SystemTime::now();
    let retain_time = now - Duration::from_secs(KEEP_SEGMENT_SECS);

    let mut outdate_offsets = vec![];

    for item in segment_infos {
        if item.mod_time <= retain_time && item.offset != active_segment_offset {
            outdate_offsets.push(item.offset)
        }
    }

    outdate_offsets
}

#[derive(Debug, Eq)]
pub struct SegmentInfo {
    pub offset: u64,
    pub mod_time: SystemTime,
}

impl SegmentInfo {
    pub fn new(offset: u64, mod_time: SystemTime) -> Self {
        SegmentInfo { offset, mod_time }
    }
}

impl PartialEq for SegmentInfo {
    fn eq(&self, other: &Self) -> bool {
        self.offset == other.offset
    }
}

impl PartialOrd for SegmentInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.offset.partial_cmp(&other.offset)
    }
}

impl Ord for SegmentInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.offset.cmp(&other.offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_nearest_offset() {
        let mod_time = SystemTime::now();

        let a = vec![
            SegmentInfo::new(2, mod_time),
            SegmentInfo::new(4, mod_time),
            SegmentInfo::new(6, mod_time),
            SegmentInfo::new(8, mod_time),
            SegmentInfo::new(10, mod_time),
        ];

        let rst = get_floor_offset(3, &a);
        assert_eq!(2, rst.unwrap());

        let rst = get_floor_offset(1, &a);
        assert_eq!(None, rst);

        let rst = get_floor_offset(5, &a);
        assert_eq!(4, rst.unwrap());

        let rst = get_floor_offset(17, &a);
        assert_eq!(10, rst.unwrap());
    }

    #[test]
    fn test_find_outdate_segment() {
        let mod_time = SystemTime::now();

        let segment_infos = vec![
            SegmentInfo::new(0, mod_time - Duration::from_secs(10 * KEEP_SEGMENT_SECS)),
            SegmentInfo::new(2, mod_time - Duration::from_secs(20 * KEEP_SEGMENT_SECS)),
            SegmentInfo::new(4, mod_time - Duration::from_secs(5 * KEEP_SEGMENT_SECS)),
            SegmentInfo::new(5, mod_time),
        ];

        let rst = find_outdate_segment(&segment_infos, 4);

        assert_eq!(2, rst.len())
    }

    #[test]
    fn test_get_oldest_offset() {
        let mod_time = SystemTime::now();

        let segment_infos = vec![
            SegmentInfo::new(0, mod_time - Duration::from_secs(10 * KEEP_SEGMENT_SECS)),
            SegmentInfo::new(2, mod_time - Duration::from_secs(20 * KEEP_SEGMENT_SECS)),
            SegmentInfo::new(4, mod_time - Duration::from_secs(5 * KEEP_SEGMENT_SECS)),
            SegmentInfo::new(5, mod_time),
        ];

        let rst = get_oldest_offset(&segment_infos);

        assert_eq!(2, rst.unwrap());
    }
}
