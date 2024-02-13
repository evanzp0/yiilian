use bytes::Bytes;
use yiilian_core::{common::error::Error, data::BencodeData};

pub const UT_METADATA_NAME: &str = "ut_metadata";

#[derive(Debug)]
pub enum UtMetadata {
    Request { piece: i32 },
    Reject { piece: i32 },
    Data { piece: i32, total_size: i32, block: Bytes}
}

impl TryFrom<Bytes> for UtMetadata {
    type Error = Error;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        let value: BencodeData = value.into();
        let value = value.as_map()?;

        todo!()
    }
}