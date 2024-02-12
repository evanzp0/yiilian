use bytes::Bytes;
use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum MessageId {
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
pub enum PeerMessage {
    KeepAlive,
    Bitfield {
        len: u32,
        bitfield: Bytes,
    },
    Have {
        len: u32,
        index: u32,
    },
    Request {
        len: u32,
        index: u32,
        offset_begin: u32,
        offset_length: u32,
    },
    Cancel {
        len: u32,
        index: u32,
        offset_begin: u32,
        offset_length: u32,
    },
    Piece {
        len: u32,
        index: u32,
        offset_begin: u32,
        offset_length: u32,
        piece: Bytes,
    },
    NotInterested {
        len: u32,
    },
    Interested {
        len: u32,
    },
    UnChoke {
        len: u32,
    },
    Choke {
        len: u32,
    },
    Extended {
        len: u32,
        ext_msg_id: u8,
        payload: Bytes,
    }
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
            Self::Extended{ .. } => Some(MessageId::Extended.into()),
        }
    }

    pub fn verify(bytes: &[u8]) -> bool {
        if bytes.len() < 4 {
            return false;
        }

        let payload_len = u32::from_be_bytes(
            bytes[0..4]
                .try_into()
                .expect("Can't convert bytes to [u8; 4]"),
        );

        if bytes[4..].len() != payload_len as usize {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
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
    }
}
