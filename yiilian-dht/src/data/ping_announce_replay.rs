use std::{collections::HashMap, net::SocketAddr};

use anyhow::anyhow;
use bytes::Bytes;
use yiilian_core::{YiiLianError, frame::Frame, extract_bencode_field_from_map};

use crate::{build_frame_common_field, transaction::TransactionId, common::Id, extract_frame_common_field};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PingOrAnnounceReply {
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
}
impl TryFrom<Frame> for PingOrAnnounceReply {
    type Error = YiiLianError;

    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        let (t, v, ip, ro) = extract_frame_common_field!(frame);

        if !frame.verify_items(&[("y", "r")]) {
            return Err(YiiLianError::FrameParse(anyhow!("not valid frame for PingFeedback, frame: {frame}")))
        }
        
        let a = frame.extract_dict("r")?.as_map()?;
        let id = extract_bencode_field_from_map!(a, "id", frame)?.into();

        Ok(PingOrAnnounceReply { t, v, ip, ro, id })
    }
}

impl From<&PingOrAnnounceReply> for Frame {
    fn from(value: &PingOrAnnounceReply) -> Self {
        let mut rst: HashMap<Bytes, Frame> = HashMap::new();
        build_frame_common_field!(rst, value);

        rst.insert("y".into(), "r".into());

        let mut r: HashMap<Bytes, Frame> = HashMap::new();
        r.insert("id".into(), value.id.get_bytes().into());
        
        rst.insert("r".into(), r.into());
        
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
        let af = PingOrAnnounceReply {
            t: "t1".into(),
            v: Some("v1".into()),
            ip: Some(bytes_to_sockaddr(&vec![127, 0, 0, 1, 0,80]).unwrap().into()),
            ro: Some(1),
            id: "id000000000000000001".into(),
        };
        let rst: Frame = (&af).into();

        let data = b"d1:t2:t11:y1:r1:rd2:id20:id000000000000000001e2:roi1e2:ip6:\x7f\0\0\x01\0\x501:v2:v1e";
        let data_frame = decode(data.as_slice().into()).unwrap();
        assert_eq!(data_frame, rst);

        let rst: PingOrAnnounceReply = data_frame.try_into().unwrap();
        assert_eq!(af, rst);
    }
}