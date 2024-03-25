use std::net::SocketAddr;

use bytes::{BufMut, Bytes, BytesMut};
use yiilian_core::common::{error::Error, util::{bytes_to_sockaddr, sockaddr_to_bytes}};

/// [0]: try_times
/// [1]: type, 0 - Normal, 1 - GetPeers, 2 - AnnouncePeer
/// [2..22]: infohash
/// [22..]: addr
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoMessage {
    pub try_times: u8,
    pub info_type: MessageType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageType {
    Normal([u8; 20]),
    GetPeers {info_hash: [u8; 20], addr: SocketAddr},
    AnnouncePeer {info_hash: [u8; 20], addr: SocketAddr}
}

impl From<InfoMessage> for Bytes {
    fn from(value: InfoMessage) -> Self {
        let mut rst = BytesMut::new();
        rst.put_u8(value.try_times);
        
        match value.info_type {
            MessageType::Normal(info_hash) => {
                rst.put_u8(0);
                rst.extend_from_slice(&info_hash);
            },
            MessageType::GetPeers { info_hash, addr } => {
                rst.put_u8(1);
                rst.extend_from_slice(&info_hash);
                rst.extend_from_slice(&sockaddr_to_bytes(&addr));
            },
            MessageType::AnnouncePeer { info_hash, addr } => {
                rst.put_u8(2);
                rst.extend_from_slice(&info_hash);
                rst.extend_from_slice(&sockaddr_to_bytes(&addr));
            },
        }

        rst.into()
    }
}

impl TryFrom<&[u8]> for InfoMessage {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < 21 {
            return Err(Error::new_decode(&format!("Bytes too short to InfoMessage: {}", value.len())));
        }

        let try_times: u8 = value[0]; 
        let m_type: u8 = value[1];

        let info_hash = value[2..22]
            .try_into()
            .map_err(|error| Error::new_decode(&format!("Decode info_hash error: {:?}", error)))?;

        match m_type {
            0 => {
                let rst = InfoMessage {
                    try_times,
                    info_type: MessageType::Normal(info_hash),
                };
                Ok(rst)
            },
            1 => {
                let addr = bytes_to_sockaddr(&value[22..])?;

                let rst = InfoMessage {
                    try_times,
                    info_type: MessageType::GetPeers{info_hash, addr},
                };
                Ok(rst)
            },
            2 => {
                let addr = bytes_to_sockaddr(&value[22..])?;

                let rst = InfoMessage {
                    try_times,
                    info_type: MessageType::GetPeers{info_hash, addr},
                };
                Ok(rst)
            },
            3..=u8::MAX => {
                Err(Error::new_decode(&format!("Decode info_hash not support type: {:?}", value)))?
            },
        }
    }
}

#[cfg(test)]
mod tests {

    use std::net::SocketAddr;

    use super::*;

    #[test]
    fn test_encode() {
        let info_hash = *b"00000000000000000001";
        let addr: SocketAddr = "127.0.0.1:80".parse().unwrap();
        let data = InfoMessage {
            try_times: 1,
            info_type: MessageType::GetPeers {info_hash, addr},
        };

        let rst: Bytes = data.clone().into();
        let d_rst = b"\x01\x0100000000000000000001\x7f\0\0\x01\0P";
        assert_eq!(d_rst.as_slice(), rst);


        let rst: InfoMessage = d_rst.as_slice().try_into().unwrap();

        assert_eq!(rst, data)
    }
}