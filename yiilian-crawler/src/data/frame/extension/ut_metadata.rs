use std::collections::BTreeMap;

use bytes::{Bytes, BytesMut};
use yiilian_core::{
    common::error::Error,
    data::{decode_dict, BencodeData, Encode},
};

use crate::data::frame::PeerMessage;

pub const UT_METADATA_NAME: &str = "ut_metadata";
pub const UT_METADATA_ID: u8 = 1;
pub const METADATA_PIECE_BLOCK: usize = 16 * 1024;

#[derive(Debug, Clone)]
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

impl UtMetadata {
    pub fn into_peer_message(self, message_id: u8) -> PeerMessage {
        let payload: Bytes = self.into();

        PeerMessage::Extended {
            ext_msg_id: message_id,
            payload,
        }
    }
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

                        Ok(UtMetadata::Data {
                            piece,
                            total_size,
                            block: body,
                        })
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

impl From<UtMetadata> for Bytes {
    fn from(value: UtMetadata) -> Self {
        match value {
            UtMetadata::Request { piece } => {
                let mut rst: BTreeMap<Bytes, BencodeData> = BTreeMap::new();
                rst.insert("msg_type".into(), 0.into());
                rst.insert("piece".into(), piece.into());

                rst.encode()
            }
            UtMetadata::Data {
                piece,
                total_size,
                block,
            } => {
                let mut rst: BTreeMap<Bytes, BencodeData> = BTreeMap::new();
                rst.insert("msg_type".into(), 1.into());
                rst.insert("piece".into(), piece.into());
                rst.insert("total_size".into(), total_size.into());

                let mut bytes = BytesMut::new();
                bytes.extend(rst.encode());
                bytes.extend(block);

                bytes.into()
            }
            UtMetadata::Reject { piece } => {
                let mut rst: BTreeMap<Bytes, BencodeData> = BTreeMap::new();
                rst.insert("msg_type".into(), 2.into());
                rst.insert("piece".into(), piece.into());

                rst.encode()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codec() {
        let message = UtMetadata::Data {
            piece: 0,
            total_size: 100,
            block: b"abcd"[..].into(),
        };

        let data: Bytes = message.into();
        let message: UtMetadata = data.clone().try_into().unwrap();
        let data1: Bytes = message.into();

        assert_eq!(data, data1);
    }
}
