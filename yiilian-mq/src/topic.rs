use std::path::PathBuf;

use crate::{consumer_offsets::ConsumerOffsets, segment::active_segment::ActiveSegment};


pub struct Topic {
    name: String,
    path: PathBuf,
    active_segment: ActiveSegment,
    consumer_offsets: ConsumerOffsets,
    segment_offsets: Vec<u64>,
}

impl Topic {
    pub fn new(name: String, mut base_path: PathBuf) -> Self {
        base_path.push(name.clone());

        let active_segment = ActiveSegment::new(offset, base_path);


        Topic {
            name,
            path: base_path,
        }
    }
}