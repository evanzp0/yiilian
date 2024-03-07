use std::{
    fs::{self, OpenOptions},
    path::PathBuf,
};

use chrono::Utc;
use yiilian_core::common::{
    error::Error,
    util::atoi,
};

use crate::{
    consumer_offsets::ConsumerOffsets, message::{self, in_message::InMessage, Message, MESSAGE_PREFIX_LEN}, segment::active_segment::ActiveSegment,
};

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
                            println!("{}", file_name);
                            
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

    pub fn push_message(&mut self, message: InMessage) -> Result<(), Error> {
        let message_size = 20 + message.0.len() + MESSAGE_PREFIX_LEN;
        let enough_space = self.active_segment.enough_space(message_size);

        if !enough_space {
            let new_offset = self.active_segment.get_last_message_offset().unwrap_or(0);

            let active_segment = ActiveSegment::new(new_offset, self.path.clone())?;

            self.segment_offsets.push(new_offset);

            self.active_segment = active_segment;
        }

        let new_offset = self.active_segment.get_last_message_offset().unwrap_or(0);
        let message = Message::new(new_offset, Utc::now().timestamp_millis(), message.0);

        self.active_segment.push_message(message)
    }
}
