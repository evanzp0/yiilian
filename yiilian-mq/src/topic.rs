use std::{
    fs::{self, OpenOptions},
    path::PathBuf,
    time::SystemTime,
};

use chrono::Utc;
use yiilian_core::common::{error::Error, util::atoi};

use crate::{
    consumer_offsets::ConsumerOffsets,
    message::{in_message::InMessage, Message, MESSAGE_PREFIX_LEN},
    segment::{active_segment::ActiveSegment, poll_message},
};

const KEEP_SEGMENT_SECS: u64 = 60;

pub struct Topic {
    name: String,
    path: PathBuf,
    active_segment: ActiveSegment,
    consumer_offsets: ConsumerOffsets,
    segment_offsets: Vec<SegmentInfo>,
}

impl Topic {
    pub fn new(name: &str, path: PathBuf) -> Result<Self, Error> {
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

                            match segment_offsets.binary_search(&segment_info) {
                                Ok(_pos) => {}
                                Err(pos) => segment_offsets.insert(pos, segment_info),
                            }
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
        let active_segment = ActiveSegment::new(last_segment_offset, path.clone())?;

        let consumer_offsets_path: PathBuf = {
            let mut p = path.clone();
            p.push("_consumer_offsets");
            p
        };
        let consumer_offsets_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&consumer_offsets_path)
            .unwrap();
        let consumer_offsets = ConsumerOffsets::new_from_file(consumer_offsets_file)?;

        Ok(Topic {
            name: name.to_owned(),
            path,
            active_segment,
            consumer_offsets,
            segment_offsets,
        })
    }

    pub fn segment_offsets(&self) -> &Vec<SegmentInfo> {
        &self.segment_offsets
    }

    pub fn push_message(&mut self, message: InMessage) -> Result<(), Error> {
        let message_size = 20 + message.0.len() + MESSAGE_PREFIX_LEN;

        let enough_space = self.active_segment.enough_space(message_size);

        if !enough_space {
            let new_offset = self.active_segment.get_next_offset();
            let active_segment = ActiveSegment::new(new_offset, self.path.clone())?;
            let segment_info = SegmentInfo::new(new_offset, SystemTime::now());
            self.segment_offsets.push(segment_info);
            self.active_segment = active_segment;
        }

        let new_offset = self.active_segment.get_next_offset();
        let message = Message::new(new_offset, Utc::now().timestamp_millis(), message.0);

        self.active_segment.push_message(message)
    }

    pub fn poll_message(&mut self, customer_name: &str) -> Option<Message> {
        let (segment_offset, target_offset) =
            if let Some(mut target_offset) = self.consumer_offsets.get(customer_name) {
                target_offset += 1;

                // println!("{:?}", (self.get_segment_offset(target_offset), target_offset));

                (self.get_segment_offset(target_offset), target_offset)
            } else {
                if let Some(segment_info) = self.segment_offsets.get(0) {
                    (segment_info.offset, segment_info.offset)
                } else {
                    return None;
                }
            };

        if let Ok(message) = poll_message(&self.path, segment_offset, target_offset) {
            if message.is_some() {
                self.consumer_offsets
                    .insert(customer_name, target_offset)
                    .ok();
            }

            message
        } else {
            None
        }
    }

    pub fn get_segment_offset(&self, target_offset: u64) -> u64 {
        get_nearest_offset(target_offset, &self.segment_offsets)
    }

    pub fn purge_segment(&mut self) {
        todo!()
    }
}

fn get_nearest_offset(target_offset: u64, array: &Vec<SegmentInfo>) -> u64 {
    if array.len() == 0 {
        return 0;
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
            return mid_offset;
        } else if mid_offset < target_offset {
            left = mid + 1;
        } else if mid_offset > target_offset {
            right = mid - 1;
        }
    }

    let left = if left <= 0 { 0 } else { left - 1 };

    mid_offset = array
        .get(left as usize)
        .expect("Not found mid item in LogIndex")
        .offset;

    mid_offset
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

        let a = vec![SegmentInfo::new(0, mod_time), SegmentInfo::new(2, mod_time), SegmentInfo::new(4, mod_time)];

        let rst = get_nearest_offset(3, &a);
        assert_eq!(2, rst);

        let rst = get_nearest_offset(1, &a);
        assert_eq!(0, rst);

        let rst = get_nearest_offset(5, &a);
        assert_eq!(4, rst);

        let rst = get_nearest_offset(9, &a);
        assert_eq!(4, rst);

        let a = vec![SegmentInfo::new(2, mod_time), SegmentInfo::new(4, mod_time), SegmentInfo::new(6, mod_time)];

        let rst = get_nearest_offset(1, &a);
        assert_eq!(2, rst);
    }
}
