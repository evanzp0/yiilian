use bytes::{BufMut, Bytes, BytesMut};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use yiilian_core::common::{error::Error, util::be_bytes_to_u32};

const KEEP_ALIVE_MESSAGE: [u8;4] = [0, 0, 0, 0];
pub const MESSAGE_LEN_PREFIX: usize = 4;

#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
enum MessageId {
    Choke = 0,
    UnChoke = 1,
    Interested = 2,
    NotInterested = 3,
    Have = 4,
    Bitfield = 5,
    Request = 6,
    Piece = 7,
    Cancel = 8,
    Extended = 20,
}

#[allow(unused)]
#[derive(Debug, PartialEq, Eq)]
pub enum PeerMessage {
    KeepAlive,
    Bitfield {
        bitfield: Bytes,
    },
    Have {
        index: u32,
    },
    Request {
        index: u32,
        offset_begin: u32,
        offset_length: u32,
    },
    Cancel {
        index: u32,
        offset_begin: u32,
        offset_length: u32,
    },
    Piece {
        index: u32,
        offset_begin: u32,
        piece: Bytes,
    },
    NotInterested,
    Interested,
    UnChoke,
    Choke,
    Extended {
        ext_msg_id: u8,
        payload: Bytes,
    },
}

impl PeerMessage {
    pub fn get_message_id(&self) -> Option<u8> {
        match self {
            Self::KeepAlive => None,
            Self::Bitfield { .. } => Some(MessageId::Bitfield.into()),
            Self::Have { .. } => Some(MessageId::Have.into()),
            Self::Request { .. } => Some(MessageId::Request.into()),
            Self::Cancel { .. } => Some(MessageId::Cancel.into()),
            Self::Piece { .. } => Some(MessageId::Piece.into()),
            Self::NotInterested { .. } => Some(MessageId::NotInterested.into()),
            Self::Interested { .. } => Some(MessageId::Interested.into()),
            Self::UnChoke { .. } => Some(MessageId::UnChoke.into()),
            Self::Choke { .. } => Some(MessageId::Choke.into()),
            Self::Extended { .. } => Some(MessageId::Extended.into()),
        }
    }

    pub fn verify(bytes: &[u8]) -> bool {
        if bytes.len() < 4 {
            return false;
        }

        let payload_len = if let Ok(val) = be_bytes_to_u32(&bytes[0..4]) {
            val
        } else {
            return false
        };

        if bytes[4..].len() != payload_len as usize {
            return false;
        }

        true
    }
}

impl TryFrom<Bytes> for PeerMessage {
    type Error = Error;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        if !PeerMessage::verify(&value) {
            Err(Error::new_frame(
                None,
                Some(format!("Can't Convert Bytes to PeerMessage: {:?}", value)),
            ))?
        }

        if value[0..4] == KEEP_ALIVE_MESSAGE 
            && value.len() == KEEP_ALIVE_MESSAGE.len() 
        {
            return Ok(PeerMessage::KeepAlive);
        }

        let message_id = {
            let rst = MessageId::try_from(value[4] as u8);
            if let Ok(val) = rst {
                val
            } else {
                Err(Error::new_frame(
                    None,
                    Some(format!(
                        "Can't Convert Bytes to PeerMessage, message_id is not support: {:?}",
                        value
                    )),
                ))?
            }
        };

        let peer_message = match message_id {
            MessageId::Choke => PeerMessage::Choke,
            MessageId::UnChoke => PeerMessage::UnChoke,
            MessageId::Interested => PeerMessage::Interested,
            MessageId::NotInterested => PeerMessage::NotInterested,
            MessageId::Have => {
                let index = be_bytes_to_u32(&value[5..])?;
                PeerMessage::Have { index }
            }
            MessageId::Bitfield => {
                let bitfield: Bytes = value[5..].to_owned().into();
                PeerMessage::Bitfield { bitfield }
            }
            MessageId::Request => {
                let index = be_bytes_to_u32(&value[5..9])?;
                let offset_begin = be_bytes_to_u32(&value[9..13])?;
                let offset_length = be_bytes_to_u32(&value[13..17])?;
                PeerMessage::Request {
                    index,
                    offset_begin,
                    offset_length,
                }
            }
            MessageId::Piece => {
                let index = be_bytes_to_u32(&value[5..9])?;
                let offset_begin = be_bytes_to_u32(&value[9..13])?;
                let piece: Bytes = value[13..].to_owned().into();
                PeerMessage::Piece {
                    index,
                    offset_begin,
                    piece,
                }
            }
            MessageId::Cancel => {
                let index = be_bytes_to_u32(&value[5..9])?;
                let offset_begin = be_bytes_to_u32(&value[9..13])?;
                let offset_length = be_bytes_to_u32(&value[13..17])?;
                PeerMessage::Cancel {
                    index,
                    offset_begin,
                    offset_length,
                }
            }
            MessageId::Extended => {
                let ext_msg_id = value[5];
                let payload: Bytes = value[6..].to_owned().into();
                PeerMessage::Extended {
                    ext_msg_id,
                    payload,
                }
            }
        };

        Ok(peer_message)
    }
}

impl From<PeerMessage> for Bytes {
    fn from(value: PeerMessage) -> Self {
        let message_id = value.get_message_id();

        match value {
            PeerMessage::KeepAlive => {
                KEEP_ALIVE_MESSAGE[..].into()
            }
            PeerMessage::Bitfield { bitfield } => {
                let len = 1 + bitfield.len();
                let mut rst = BytesMut::with_capacity(4 + len);
                
                rst.extend((len as u32).to_be_bytes());
                rst.put_u8(message_id.expect("message_id can't be none"));
                rst.extend(bitfield);

                rst.into()
            }
            PeerMessage::Have { index } => {
                let len = 5;
                let mut rst = BytesMut::with_capacity(4 + len);

                rst.extend((len as u32).to_be_bytes());
                rst.put_u8(message_id.expect("message_id can't be none"));
                rst.extend(index.to_be_bytes());

                rst.into()
            }
            PeerMessage::Request { index, offset_begin, offset_length } => {
                let len = 13;
                let mut rst = BytesMut::with_capacity(4 + len);

                rst.extend((len as u32).to_be_bytes());
                rst.put_u8(message_id.expect("message_id can't be none"));
                rst.extend(index.to_be_bytes());
                rst.extend(offset_begin.to_be_bytes());
                rst.extend(offset_length.to_be_bytes());

                rst.into()
            }
            PeerMessage::Cancel { index, offset_begin, offset_length } => {
                let len = 13;
                let mut rst = BytesMut::with_capacity(4 + len);

                rst.extend((len as u32).to_be_bytes());
                rst.put_u8(message_id.expect("message_id can't be none"));
                rst.extend(index.to_be_bytes());
                rst.extend(offset_begin.to_be_bytes());
                rst.extend(offset_length.to_be_bytes());

                rst.into()
            }
            PeerMessage::Piece { index, offset_begin, piece } => {
                let len = 9 + piece.len();
                let mut rst = BytesMut::with_capacity(4 + len);

                rst.extend((len as u32).to_be_bytes());
                rst.put_u8(message_id.expect("message_id can't be none"));
                rst.extend(index.to_be_bytes());
                rst.extend(offset_begin.to_be_bytes());
                rst.extend(piece);

                rst.into()
            }
            PeerMessage::NotInterested => {
                let len = 1;
                let mut rst = BytesMut::with_capacity(4 + len);

                rst.extend((len as u32).to_be_bytes());
                rst.put_u8(message_id.expect("message_id can't be none"));

                rst.into()
            }
            PeerMessage::Interested => {
                let len = 1;
                let mut rst = BytesMut::with_capacity(4 + len);

                rst.extend((len as u32).to_be_bytes());
                rst.put_u8(message_id.expect("message_id can't be none"));

                rst.into()
            }
            PeerMessage::UnChoke => {
                let len = 1;
                let mut rst = BytesMut::with_capacity(4 + len);

                rst.extend((len as u32).to_be_bytes());
                rst.put_u8(message_id.expect("message_id can't be none"));

                rst.into()
            }
            PeerMessage::Choke => {
                let len = 1;
                let mut rst = BytesMut::with_capacity(4 + len);

                rst.extend((len as u32).to_be_bytes());
                rst.put_u8(message_id.expect("message_id can't be none"));

                rst.into()
            }
            PeerMessage::Extended { ext_msg_id, payload } => {
                let len = 2 + payload.len();
                let mut rst = BytesMut::with_capacity(4 + len);

                rst.extend((len as u32).to_be_bytes());
                rst.put_u8(message_id.expect("message_id can't be none"));
                rst.put_u8(ext_msg_id);
                rst.extend(payload);

                rst.into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use crate::data::frame::PeerMessage;

    #[test]
    fn test_verify() {
        let bytes: Vec<u8> = vec![0, 0, 0, 2, b'a', b'b'];
        assert_eq!(true, PeerMessage::verify(bytes.as_slice()));

        let bytes: Vec<u8> = vec![0, 0, 0, 2, b'a'];
        assert_eq!(false, PeerMessage::verify(bytes.as_slice()));

        let bytes: Vec<u8> = vec![0, 0, 0, 2, b'a', b'b', b'c'];
        assert_eq!(false, PeerMessage::verify(bytes.as_slice()));

        let bytes: Vec<u8> = vec![0, 0, 0];
        assert_eq!(false, PeerMessage::verify(bytes.as_slice()));

        let bytes: Vec<u8> = vec![0, 0, 0, 0];
        assert_eq!(true, PeerMessage::verify(bytes.as_slice()));
    }

    #[test]
    fn test_parse() {
        let bytes: Bytes = vec![0, 0, 0, 4, 20, 0, b'd', b'1'].into();
        let rst: PeerMessage = bytes.try_into().unwrap();

        let ext_handshake_msg = PeerMessage::Extended { ext_msg_id: 0, payload: b"d1"[..].into() };

        assert_eq!(ext_handshake_msg, rst);
    }

    #[test]
    fn test_encode() {
        let ext_handshake_msg = PeerMessage::Extended { ext_msg_id: 0, payload: b"d1"[..].into() };
        let rst: Bytes = ext_handshake_msg.into();

        let bytes: Bytes = vec![0, 0, 0, 4, 20, 0, b'd', b'1'].into();
        assert_eq!(bytes, rst);
    }
}
