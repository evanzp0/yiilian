use std::{collections::HashMap, net::SocketAddr};

use bytes::Bytes;
use yiilian_core::common::error::Error;

use yiilian_core::data::BencodeFrame as Frame;

use crate::{common::Id, gen_frame_common_field, transaction::TransactionId};

use super::util::extract_frame_common_field;

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

impl FindNode {
    pub fn new(
        id: Id,
        target: Id,
        t: TransactionId,
        v: Option<Bytes>,
        ip: Option<SocketAddr>,
        ro: Option<i32>,
    ) -> Self {
        Self {
            id,
            target,
            t,
            v,
            ip,
            ro,
        }
    }
}

impl TryFrom<Frame> for FindNode {
    type Error = Error;

    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        let (t, v, ip, ro) = extract_frame_common_field(&frame)?;
        if !frame.verify_items(&[("y", "q"), ("q", "find_node")]) {
            return Err(Error::new_frame(
                None,
                Some(format!("Invalid frame for FindNode, frame: {frame}")),
            ));
        }

        let a = frame.get_dict_item("a").ok_or(Error::new_frame(
            None,
            Some(format!("Field 'a' not found in frame: {frame}")),
        ))?;
        let id: Id = a
            .get_dict_item("id")
            .ok_or(Error::new_frame(
                None,
                Some(format!("Field 'id' not found in frame: {frame}")),
            ))?
            .as_bstr()?
            .to_owned()
            .try_into()?;

        let target: Id = a
            .get_dict_item("target")
            .ok_or(Error::new_frame(
                None,
                Some(format!("Field 'target' not found in frame: {frame}")),
            ))?
            .as_bstr()?
            .to_owned()
            .try_into()?;

        Ok(FindNode {
            t,
            v,
            ip,
            ro,
            id,
            target,
        })
    }
}

impl From<FindNode> for Frame {
    fn from(value: FindNode) -> Self {
        let mut rst: HashMap<Bytes, Frame> = HashMap::new();
        gen_frame_common_field!(rst, value);

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
    use yiilian_core::data::decode;

    use super::*;

    #[test]
    fn test() {
        let af = FindNode::new(
            "id000000000000000001".try_into().unwrap(),
            "info0000000000000001".try_into().unwrap(),
            "t1".into(),
            Some("v1".into()),
            Some("127.0.0.1:80".parse().unwrap()),
            Some(1),
        );
        let rst: Frame = af.clone().into();

        let data = b"d2:ip6:\x7f\0\0\x01\0\x501:t2:t12:roi1e1:ad2:id20:id0000000000000000016:target20:info0000000000000001e1:y1:q1:q9:find_node1:v2:v1e";
        let data_frame = decode(data.as_slice().into()).unwrap();
        assert_eq!(data_frame, rst);

        let rst: FindNode = data_frame.try_into().unwrap();
        assert_eq!(af, rst);
    }
}
