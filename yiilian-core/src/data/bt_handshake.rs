use bytes::{BufMut, Bytes, BytesMut};
use crate::common::error::Error;

const HANDSHAKE_PREFIX: &'static [u8] = b"BitTorrent protocol";
pub const HANDSHAKE_LEN: usize = 68;

pub const MESSAGE_EXTENSION_ENABLE: [u8; 8] = [0, 0, 0, 0, 0, 0x10, 0, 0];

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct BtHandshake {
    prefix_len: u8,
    prefix: &'static [u8],
    reserved_byte: Bytes,
    info_hash: Bytes,
    peer_id: Bytes,
}

impl BtHandshake {
    pub fn new(reserved_byte: &[u8], info_hash: &[u8], peer_id: &[u8]) -> Self {
        BtHandshake {
            prefix_len: HANDSHAKE_PREFIX.len() as u8,
            prefix: HANDSHAKE_PREFIX,
            reserved_byte: reserved_byte.to_owned().into(),
            info_hash: info_hash.to_owned().into(),
            peer_id: peer_id.to_owned().into(),
        }
    }

    pub fn verify(data: &[u8]) -> bool {
        let prefix_len = HANDSHAKE_PREFIX.len() as u8;

        if data.len() == HANDSHAKE_LEN
            && data[0] == prefix_len
            && &data[1..(prefix_len + 1).into()] == HANDSHAKE_PREFIX
            && data[25] & 0x10 == 0x10
        {
            true
        } else {
            false
        }
            
    }

    pub fn info_hash(&self) -> &Bytes {
        &self.info_hash
    }

    pub fn peer_id(&self) -> &Bytes {
        &self.peer_id
    }
}

impl From<&BtHandshake> for Bytes {
    fn from(value: &BtHandshake) -> Self {
        value.to_owned().into()
    }
}

impl From<BtHandshake> for Bytes {
    fn from(value: BtHandshake) -> Self {
        let mut buf = Vec::with_capacity(HANDSHAKE_LEN);

        buf.push(value.prefix_len);
        buf.extend(value.prefix);
        buf.extend(value.reserved_byte);
        buf.extend(value.info_hash);
        buf.extend(value.peer_id);

        buf.into()
    }
}

impl TryFrom<Bytes> for BtHandshake {
    type Error = Error;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        if !BtHandshake::verify(&value) {
            Err(Error::new_frame(
                None,
                Some(format!(
                    "Can't Convert Bytes to Handshake: {:?}",
                    value
                )),
            ))?
        }

        Ok(BtHandshake::new(&value[20..28], &value[28..48], &value[48..]))
    }
}

impl TryFrom<BytesMut> for BtHandshake {
    type Error = Error;

    fn try_from(value: BytesMut) -> Result<Self, Self::Error> {
        if !BtHandshake::verify(&value) {
            Err(Error::new_frame(
                None,
                Some(format!(
                    "Can't Convert Bytes to Handshake: {:?}",
                    value
                )),
            ))?
        }

        Ok(BtHandshake::new(&value[20..28], &value[28..48], &value[48..]))
    }
}

impl From<BtHandshake> for BytesMut {
    fn from(value: BtHandshake) -> Self {
        let mut buf = BytesMut::with_capacity(HANDSHAKE_LEN);

        buf.put_u8(value.prefix_len);
        buf.extend(value.prefix);
        buf.extend(value.reserved_byte);
        buf.extend(value.info_hash);
        buf.extend(value.peer_id);

        buf
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_codec() {
        let info_hash = b"00000000000000000001";
        let peer_id = b"00000000000000000002";
        let hs1 = BtHandshake::new(&MESSAGE_EXTENSION_ENABLE, info_hash, peer_id);
        let data: Bytes = hs1.clone().into();
        let hs2: BtHandshake = data.try_into().unwrap();

        assert_eq!(hs1, hs2);
    }

    #[test]
    fn test_verify() {
        let info_hash = b"00000000000000000001";
        let peer_id = b"00000000000000000002";
        let hs = BtHandshake::new(&MESSAGE_EXTENSION_ENABLE, info_hash, peer_id);
        let mut data: BytesMut = hs.clone().try_into().unwrap();

        assert_eq!(true, BtHandshake::verify(&data));
        assert_eq!(false, BtHandshake::verify(&data[0..data.len() - 1]));

        data[0] = 17;
        assert_eq!(false, BtHandshake::verify(&data));
        data[0] = 19;

        data[1] = b'a';
        assert_eq!(false, BtHandshake::verify(&data));
        data[1] = b'B';

        data[25] = 0;
        assert_eq!(false, BtHandshake::verify(&data));
    }
}
