use bytes::Bytes;
use yiilian_core::{
    common::error::Error,
    data::{decode_dict, BencodeData},
};

pub const UT_METADATA_NAME: &str = "ut_metadata";

#[derive(Debug)]
pub enum UtMetadata {
    Request {
        piece: i32,
    },
    Reject {
        piece: i32,
    },
    Data {
        piece: i32,
        total_size: i32,
        block: Bytes,
    },
}

impl TryFrom<Bytes> for UtMetadata {
    type Error = Error;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        let (header, index) = decode_dict(&value, 0)?;
        let body: Bytes = value[index..].to_owned().into();

        if let BencodeData::Map(message) = header {
            if let Some(BencodeData::Int(msg_type)) = message.get(b"msg_type"[..].into()) {
                match msg_type {
                    0 => {
                        let piece = {
                            if let Some(piece) = message.get(b"piece"[..].into()) {
                                piece.as_int()?
                            } else {
                                Err(Error::new_frame(
                                    None,
                                    Some(format!(
                                        "piece item not found in ut_metadata message: {msg_type}"
                                    )),
                                ))?
                            }
                        };

                        Ok(UtMetadata::Request { piece })
                    }
                    1 => {
                        let piece = {
                            if let Some(piece) = message.get(b"piece"[..].into()) {
                                piece.as_int()?
                            } else {
                                Err(Error::new_frame(
                                    None,
                                    Some(format!(
                                        "Item 'piece' not found in ut_metadata message: {msg_type}"
                                    )),
                                ))?
                            }
                        };

                        let total_size = {
                            if let Some(total_size) = message.get(b"total_size"[..].into()) {
                                total_size.as_int()?
                            } else {
                                Err(Error::new_frame(
                                    None,
                                    Some(format!(
                                        "Item 'total_size' not found in ut_metadata message: {msg_type}"
                                    )),
                                ))?
                            }
                        };

                        Ok(UtMetadata::Data { piece, total_size, block: body })
                    }
                    2 => {
                        let piece = {
                            if let Some(piece) = message.get(b"piece"[..].into()) {
                                piece.as_int()?
                            } else {
                                Err(Error::new_frame(
                                    None,
                                    Some(format!(
                                        "piece item not found in ut_metadata message: {msg_type}"
                                    )),
                                ))?
                            }
                        };

                        Ok(UtMetadata::Reject { piece })
                    }
                    _ => Err(Error::new_frame(
                        None,
                        Some(format!("Wrong msg_type of ut_metadata message: {msg_type}")),
                    ))?,
                }
            } else {
                Err(Error::new_frame(
                    None,
                    Some(format!(
                        "Can't convert bytes to ut_metadata message: {:?}",
                        value
                    )),
                ))?
            }
        } else {
            Err(Error::new_frame(
                None,
                Some(format!(
                    "Can't convert bytes to ut_metadata message: {:?}",
                    value
                )),
            ))?
        }
    }
}
