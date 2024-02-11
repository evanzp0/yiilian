use bytes::Bytes;
use yiilian_core::common::error::Error;

pub const HANDSHAKE_PREFIX: &str = "BitTorrent protocol";
pub const HANDSHAKE_LEN: usize = 68;

pub const EXTENSION_ENABLE: [u8; 8] = [0, 0, 0, 0, 0, 16, 0, 0];

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Handshake {
    prefix_len: u8,
    prefix: &'static str,
    reserved_byte: Bytes,
    info_hash: Bytes,
    peer_id: Bytes,
}

impl Handshake {
    pub fn new(reserved_byte: &[u8], info_hash: &[u8], peer_id: &[u8]) -> Self {
        Handshake {
            prefix_len: HANDSHAKE_PREFIX.len() as u8,
            prefix: HANDSHAKE_PREFIX,
            reserved_byte: reserved_byte.to_owned().into(),
            info_hash: info_hash.to_owned().into(),
            peer_id: peer_id.to_owned().into(),
        }
    }
}

impl From<Handshake> for Bytes {
    fn from(value: Handshake) -> Self {
        let mut buf = Vec::with_capacity(HANDSHAKE_LEN);

        buf.push(value.prefix_len);
        buf.extend(value.prefix.as_bytes());
        buf.extend(value.reserved_byte);
        buf.extend(value.info_hash);
        buf.extend(value.peer_id);

        buf.into()
    }
}

impl TryFrom<Bytes> for Handshake {
    type Error = Error;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        if value.len() != HANDSHAKE_LEN {
            Err(Error::new_frame(
                None,
                Some(format!(
                    "Bytes is invalid to convert to Handshake: {:?}",
                    value
                )),
            ))?
        }

        Ok(Handshake::new(&value[20..28], &value[28..48], &value[48..]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let info_hash = b"00000000000000000001";
        let peer_id = b"00000000000000000002";
        let hs1 = Handshake::new(&EXTENSION_ENABLE, info_hash, peer_id);
        let data: Bytes = hs1.clone().into();
        let hs2: Handshake = data.try_into().unwrap();

        assert_eq!(hs1, hs2);
    }
}
