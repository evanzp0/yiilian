use std::{
    fs::{self, OpenOptions},
    path::PathBuf,
    sync::Mutex,
};

use yiilian_core::common::{
    error::{Error, Kind},
    util::atoi,
};

use crate::{
    consumer_offsets::ConsumerOffsets, message::Message, segment::active_segment::ActiveSegment,
};

pub struct Topic {
    name: String,
    path: PathBuf,
    active_segment: Mutex<ActiveSegment>,
    consumer_offsets: ConsumerOffsets,
    segment_offsets: Vec<u64>,
}

impl Topic {
    pub fn new(name: String, path: PathBuf) -> Result<Self, Error> {
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
        let active_segment = Mutex::new(active_segment);

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
            name,
            path,
            active_segment,
            consumer_offsets,
            segment_offsets,
        })
    }

    pub fn push_message(&mut self, message: Message) -> Result<(), Error> {
        let enough_space = self
            .active_segment
            .lock()
            .expect("lock active_segment")
            .enough_space(&message);

        if !enough_space {
            let new_offset = self
                .active_segment
                .lock()
                .expect("lock active_segment")
                .get_last_message_offset()
                .unwrap_or(0);

            let active_segment = {
                let a_segment = ActiveSegment::new(new_offset, self.path.clone())?;
                Mutex::new(a_segment)
            };

            self.segment_offsets.push(new_offset);

            self.active_segment = active_segment;
        }

        self.active_segment
            .lock()
            .expect("lock active_segment")
            .push_message(message)
    }
}
