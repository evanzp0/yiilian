use std::{collections::HashMap, net::SocketAddr};

use anyhow::anyhow;
use bytes::Bytes;
use yiilian_core::{YiiLianError, frame::Frame, extract_bencode_field_from_map};

use crate::{extract_frame_common_field, build_frame_common_field, transaction::TransactionId, common::Id};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetPeers {
    /// transaction_id
    pub t: TransactionId,

    /// version
    pub v: Option<Bytes>,

    /// 对方看到的我们的外网 IP
    pub ip: Option<SocketAddr>,

    /// readonly
    pub ro: Option<i32>,

    // ----------------------------

    /// sender node id
    pub id: Id,

    pub info_hash: Id,
}

impl TryFrom<Frame> for GetPeers {
    type Error = YiiLianError;

    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        let (t, v, ip, ro) = extract_frame_common_field!(frame);
        if !frame.verify_items(&[("y", "q"), ("q", "get_peers")]) {
            return Err(YiiLianError::FrameParse(anyhow!("not valid frame for GetPeers, frame: {frame}")))
        }

        let a = frame.extract_dict("a")?.as_map()?;
        let id = extract_bencode_field_from_map!(a, "id", frame)?.into();
        let info_hash = extract_bencode_field_from_map!(a, "info_hash", frame)?.into();

        Ok(GetPeers { t, v, ip, ro, id, info_hash })
    }
}

impl From<&GetPeers> for Frame {
    fn from(value: &GetPeers) -> Self {
        let mut rst: HashMap<Bytes, Frame> = HashMap::new();
        build_frame_common_field!(rst, value);
        
        rst.insert("y".into(), "q".into());
        rst.insert("q".into(), "get_peers".into());

        let mut a: HashMap<Bytes, Frame> = HashMap::new();
        a.insert("id".into(), value.id.get_bytes().into());
        a.insert("info_hash".into(), value.info_hash.get_bytes().into());

        rst.insert("a".into(), a.into());
        
        Frame::Map(rst)
    }
}

#[cfg(test)]
mod tests {

    use yiilian_core::util::bytes_to_sockaddr;

    use yiilian_core::frame::decode;

    use super::*;

    #[test]
    fn test() {
        let af = GetPeers {
            t: "t1".into(),
            v: Some("v1".into()),
            ip: Some(bytes_to_sockaddr(&vec![127, 0, 0, 1, 0,80]).unwrap().into()),
            ro: Some(1),
            id: "id000000000000000001".into(),
            info_hash: "info0000000000000001".into(),
        };
        let rst: Frame = (&af).into();
        let data = b"d1:v2:v11:t2:t12:roi1e1:y1:q2:ip6:\x7f\0\0\x01\0\x501:q9:get_peers1:ad2:id20:id0000000000000000019:info_hash20:info0000000000000001ee";
        let data_frame = decode(data.as_slice().into()).unwrap();
        assert_eq!(data_frame, rst);

        let rst: GetPeers = data_frame.try_into().unwrap();
        assert_eq!(af, rst);
    }
}