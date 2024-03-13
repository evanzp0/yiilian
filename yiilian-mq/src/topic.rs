use std::{
    fs::{self, OpenOptions},
    path::PathBuf,
};

use chrono::Utc;
use yiilian_core::common::{error::Error, util::atoi};

use crate::{
    consumer_offsets::ConsumerOffsets,
    message::{in_message::InMessage, Message, MESSAGE_PREFIX_LEN},
    segment::{active_segment::ActiveSegment, poll_message},
};

const KEEP_SEGMENT_NUMS: usize = 2;

pub struct Topic {
    name: String,
    path: PathBuf,
    active_segment: ActiveSegment,
    consumer_offsets: ConsumerOffsets,
    segment_offsets: Vec<u64>,
}

impl Topic {
    pub fn new(name: &str, path: PathBuf) -> Result<Self, Error> {
        let entries = fs::read_dir(path.clone())
            .map_err(|error| Error::new_file(Some(error.into()), None))?;

        let mut segment_offsets = vec![];
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

                            match segment_offsets.binary_search(&offset) {
                                Ok(_pos) => {}
                                Err(pos) => segment_offsets.insert(pos, offset),
                            }
                        }
                    }
                }
            }
        }

        if segment_offsets.len() == 0 {
            segment_offsets.push(0);
        }

        let last_segment_offset = segment_offsets
            .get(segment_offsets.len() - 1)
            .expect("segment_offsets should exist");
        let active_segment = ActiveSegment::new(*last_segment_offset, path.clone())?;

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

    pub fn segment_offsets(&self) -> &Vec<u64>{
        &self.segment_offsets
    }

    pub fn push_message(&mut self, message: InMessage) -> Result<(), Error> {
        let message_size = 20 + message.0.len() + MESSAGE_PREFIX_LEN;

        let enough_space = self.active_segment.enough_space(message_size);

        if !enough_space {
            let new_offset = self.active_segment.get_next_offset();
            let active_segment = ActiveSegment::new(new_offset, self.path.clone())?;
            self.segment_offsets.push(new_offset);
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
                if let Some(segment_offset) = self.segment_offsets.get(0) {
                    (*segment_offset, *segment_offset)
                } else {
                    return None
                }
            };

        if let Ok(message) = poll_message(&self.path, segment_offset, target_offset) {
            if message.is_some() {
                self.consumer_offsets.insert(customer_name, target_offset).ok();
            }

            message
        } else {
            None
        }
    }

    pub fn get_segment_offset(&self, target_offset: u64) -> u64 {
        get_nearest_offset(target_offset, &self.segment_offsets)
    }

    pub fn pruge_segment(&mut self) {
        if self.segment_offsets.len() > KEEP_SEGMENT_NUMS {
            let remove_num = self.segment_offsets.len() - KEEP_SEGMENT_NUMS;

            for i in 0.. remove_num {
                let offset_begin = self.segment_offsets.remove(i);
                let offset_end = self.segment_offsets.remove(i);
                // self.consumer_offsets.remove_by_offset(offset)

            }
        }
    }
}

fn get_nearest_offset(target_offset: u64, array: &Vec<u64>) -> u64 {
    if array.len() == 0 {
        return 0;
    }

    let mut left: i32 = 0;
    let mut right: i32 = (array.len() - 1) as i32;
    let mut mid_offset;

    while left <= right {
        let mid = left + (right - left) / 2;
        mid_offset = *array.get(mid as usize).expect("Not found mid item in LogIndex");

        if mid_offset == target_offset {
            return mid_offset;
        } else if mid_offset < target_offset {
            left = mid + 1;
        } else if mid_offset > target_offset {
            right = mid - 1;
        }
    }

    let left = if left <= 0 {
        0
    } else {
        left -1
    };

    mid_offset = *array.get(left as usize).expect("Not found mid item in LogIndex");

    mid_offset
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_nearest_offset() {
        let a = vec![0, 2, 4];

        let rst = get_nearest_offset(3, &a);
        assert_eq!(2, rst);

        let rst = get_nearest_offset(1, &a);
        assert_eq!(0, rst);

        let rst = get_nearest_offset(5, &a);
        assert_eq!(4, rst);

        let rst = get_nearest_offset(9, &a);
        assert_eq!(4, rst);

        let a = vec![2, 4, 6];

        let rst = get_nearest_offset(1, &a);
        assert_eq!(2, rst);
    }
}
