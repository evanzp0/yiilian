use bytes::{BufMut, Bytes, BytesMut};
use yiilian_core::common::error::Error;

pub const MESSAGE_PREFIX_LEN: usize = 4;
pub const MESSAGE_OFFSET_LEN: usize = 8;
pub const MESSAGE_CRC_LEN: usize = 4;
pub const MESSAGE_TIMESTAMP_LEN: usize = 8;
pub const MIN_MESSAGE_LEN: usize = MESSAGE_CRC_LEN + MESSAGE_TIMESTAMP_LEN;


/// message_len(4) + offset(8) + crc32(4) + timestamp(8) + value(x)
/// message = offset + crc + timestamp + value
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    length: usize,
    offset: u64,
    /// utc 毫秒
    timestamp: i64,
    value: Bytes,
}

impl Message {
    pub fn new(offset: u64, timestamp: i64, value: Bytes) -> Self {
        let length = 20 + value.len();

        Self {
            length,
            offset,
            timestamp,
            value,
        }
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn total_size(&self) -> usize {
        self.length + MESSAGE_PREFIX_LEN
    }

    pub fn crc(&self) -> u32 {
        crc32fast::hash(&self.value)
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn value(&self) -> &[u8] {
        &self.value
    }
}

impl From<Message> for Bytes {
    fn from(message: Message) -> Self {
        let value_len = message.value.len();
        let message_len = MESSAGE_OFFSET_LEN + MESSAGE_CRC_LEN + MESSAGE_TIMESTAMP_LEN + value_len;
        let total_len = 4 + message_len;

        let crc = crc32fast::hash(&message.value);

        let mut buf = BytesMut::with_capacity(total_len);
        buf.put_u32(message_len as u32);
        buf.put_u64(message.offset);
        buf.put_u32(crc);
        buf.put_i64(message.timestamp);
        buf.extend(message.value);

        buf.into()
    }
}


impl TryFrom<&[u8]> for Message {
    type Error = Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() < MIN_MESSAGE_LEN {
            Err(Error::new_decode(&format!("Data is too short to decode message: {:?}", data.len())))?;
        }

        let message_len = u32::from_be_bytes(data[0..4].try_into().expect("data[0..4] is not satisfy"));
        if message_len < 20 {
            Err(Error::new_decode(&format!("Decoding message is failed at verify message_size: {:?}", message_len)))?;
        }
        let total_size = 4 + message_len;

        if data.len() < total_size as usize {
            Err(Error::new_decode(&format!("Decoding message is failed at verify length: {:?}", total_size)))?;
        }

        let value_len = (message_len - 20) as usize;
        let crc = u32::from_be_bytes(data[12..16].try_into().expect("data[12..16] is not satisfy"));
        let value: Bytes = data[24..24 + value_len].to_owned().into();

        if crc32fast::hash(&value) != crc {
            Err(Error::new_decode("Decoding message is failed at verify crc"))?;
        }

        let offset = u64::from_be_bytes(data[4..12].try_into().expect("data[4..12] is not satisfy"));
        let timestamp = i64::from_be_bytes(data[16..24].try_into().expect("data[16..24] is not satisfy"));

        Ok(Message::new(offset, timestamp, value))
    }
}

impl TryFrom<Bytes> for Message {
    type Error = Error;

    fn try_from(data: Bytes) -> Result<Self, Self::Error> {
        (&data[..]).try_into()
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use chrono::Utc;

    use crate::message::MESSAGE_PREFIX_LEN;

    use super::Message;

    #[test]
    fn test() {
        let offset = 1 as u64;
        let timestamp = Utc::now().timestamp_millis();
        let value: Bytes = b"hello"[..].into();

        let msg = Message::new(offset, timestamp, value);
        let msg_len = msg.len();
        assert_eq!(25, msg_len);

        let data: Bytes = msg.clone().into();
        assert_eq!(msg_len, data.len() - MESSAGE_PREFIX_LEN);

        let msg2: Message = data.try_into().unwrap();
        assert_eq!(msg, msg2);

        let data = Bytes::from_static(&[0, 0, 0, 0]);
        if Message::try_from(data).is_ok() {
            panic!("error")
        }

        let data = Bytes::from_static(&[
            0, 0, 0, 0, 0, 0, 0, 0, 
            0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        if Message::try_from(data).is_ok() {
            panic!("error")
        }

        let data = Bytes::from_static(&[
            0, 0, 0, 28, 0, 0, 0, 0, 
            0, 0, 0, 1, 101, 34, 223, 105,
            0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ]);

        let msg = Message::try_from(data);
        assert_eq!(true, msg.is_ok());
    }
}
