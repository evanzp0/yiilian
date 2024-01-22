use std::{collections::HashMap, net::SocketAddr};

use anyhow::anyhow;
use bytes::Bytes;
use yiilian_core::{YiiLianError, frame::Frame, extract_bencode_field_from_map};

use crate::{build_frame_common_field, transaction::TransactionId, common::Id, extract_frame_common_field};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindNode {
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

    /// 要查找的目标 node id
    pub target: Id,
}

impl TryFrom<Frame> for FindNode {
    type Error = YiiLianError;

    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        let (t, v, ip, ro) = extract_frame_common_field!(frame);
        if !frame.verify_items(&[("y", "q"), ("q", "find_node")]) {
            return Err(YiiLianError::FrameParse(anyhow!("not valid frame for FindNode, frame: {frame}")))
        }

        let a = frame.extract_dict("a")?.as_map()?;
        let id = extract_bencode_field_from_map!(a, "id", frame)?.into();

        let target = extract_bencode_field_from_map!(a, "target", frame)?.into();

        Ok(FindNode { t, v, ip, ro, id, target })
    }
}

impl From<&FindNode> for Frame {
    fn from(value: &FindNode) -> Self {
        let mut rst: HashMap<Bytes, Frame> = HashMap::new();
        build_frame_common_field!(rst, value);
        
        rst.insert("y".into(), "q".into());
        rst.insert("q".into(), "find_node".into());

        let mut a: HashMap<Bytes, Frame> = HashMap::new();
        a.insert("id".into(), value.id.get_bytes().into());
        a.insert("target".into(), value.target.get_bytes().into());

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
        let af = FindNode {
            t: "t1".into(),
            v: Some("v1".into()),
            ip: Some(bytes_to_sockaddr(&vec![127, 0, 0, 1, 0,80]).unwrap().into()),
            ro: Some(1),
            id: "id000000000000000001".into(),
            target: "info0000000000000001".into(),
        };
        let rst: Frame = (&af).into();

        let data = b"d2:ip6:\x7f\0\0\x01\0\x501:t2:t12:roi1e1:ad2:id20:id0000000000000000016:target20:info0000000000000001e1:y1:q1:q9:find_node1:v2:v1e";
        let data_frame = decode(data.as_slice().into()).unwrap();
        assert_eq!(data_frame, rst);

        let rst: FindNode = data_frame.try_into().unwrap();
        assert_eq!(af, rst);
    }
}