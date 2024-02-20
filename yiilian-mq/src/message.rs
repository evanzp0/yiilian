use bytes::{BufMut, Bytes, BytesMut};
use yiilian_core::common::error::Error;
pub struct Message {
    length: usize,
    pub offset: u64,
    /// utc 毫秒
    pub timestamp: i64,
    pub value: Bytes,
}

impl Message {
    pub fn new(offset: u64, timestamp: i64, value: Bytes) -> Self {
        let length = 24 + value.len();

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
}

/// offset(8) + message_size(4) + crc32(4) + timestamp(8) + value(x)
impl From<Message> for Bytes {
    fn from(message: Message) -> Self {
        let value_size = message.value.len();
        let message_size = 12 + value_size;
        let total_size = 12 + message_size;
        let crc = crc32fast::hash(&message.value);

        let mut buf = BytesMut::with_capacity(total_size);
        buf.put_u64(message.offset);
        buf.put_u64(message_size as u64);
        buf.put_u32(crc);
        buf.put_i64(message.timestamp);
        buf.extend(message.value);

        buf.into()
    }
}

impl TryFrom<Bytes> for Message {
    type Error = Error;

    fn try_from(data: Bytes) -> Result<Self, Self::Error> {
        if data.len() < 12 {
            Err(Error::new_decode(&format!("Data is too short to decode message: {:?}", data)))?;
        }

        let message_size = u32::from_le_bytes(data[8..12].try_into().expect("data[8..12] is not satisfy"));
        let total_size = 12 + message_size;

        if data.len() < total_size as usize {
            Err(Error::new_decode(&format!("Decoding message is failed at verify length: {:?}", data)))?;
        }

        let value_size = (message_size - 12) as usize;
        let crc = u32::from_le_bytes(data[12..16].try_into().expect("data[12..16] is not satisfy"));
        let value: Bytes = data[24..value_size].to_owned().into();

        if crc32fast::hash(&value) != crc {
            Err(Error::new_decode(&format!("Decoding message is failed at verify crc: {:?}", data)))?;
        }
        let offset = u64::from_le_bytes(data[0..8].try_into().expect("data[0..8] is not satisfy"));

        let timestamp = i64::from_le_bytes(data[16..24].try_into().expect("data[16..24] is not satisfy"));

        Ok(Message::new(offset, timestamp, value))
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use chrono::{DateTime, Utc};

    #[test]
    fn test() {
        let a = Utc::now();
        println!("{}", a.timestamp());
        println!("{}", a.timestamp_millis());
        println!("{}", a.timestamp_micros());
        println!("{}", i64::MAX);
        println!("{:?}", DateTime::from_timestamp_millis(1708356965920896));
        println!("{}", size_of::<DateTime<Utc>>())
    }
}
