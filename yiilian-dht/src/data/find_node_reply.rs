use std::{collections::BTreeMap, net::SocketAddr};

use bytes::Bytes;
use yiilian_core::{common::error::Error, data::BencodeData as Frame};

use crate::{
    common::{
        {Id, ID_SIZE},
        bytes_to_nodes4,
    },
    gen_frame_common_field, merge_node_bytes,
    routing_table::Node,
    transaction::TransactionId,
};

use super::util::extract_frame_common_field;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindNodeReply {
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

    /// feedback params
    // pub nodes: Vec<Bytes>,
    pub nodes: Vec<Node>,
}

impl FindNodeReply {
    pub fn new(
        id: Id,
        nodes: Vec<Node>,
        t: TransactionId,
        v: Option<Bytes>,
        ip: Option<SocketAddr>,
        ro: Option<i32>,
    ) -> Self {
        Self {
            id,
            nodes,
            t,
            v,
            ip,
            ro,
        }
    }
}

impl TryFrom<Frame> for FindNodeReply {
    type Error = Error;

    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        let (t, v, ip, ro) = extract_frame_common_field(&frame)?;
        if !frame.verify_items(&[("y", "r")]) {
            return Err(Error::new_frame(
                None,
                Some(format!("Invalid frame for FindNodeReply, frame: {frame}")),
            ));
        }

        let r = frame.get_dict_item("r").ok_or(Error::new_frame(
            None,
            Some(format!("Field 'r' not found in frame: {frame}")),
        ))?;

        let id = r
            .get_dict_item("id")
            .ok_or(Error::new_frame(
                None,
                Some(format!("Field 'id' not found in frame: {frame}")),
            ))?
            .as_bstr()?
            .to_owned()
            .try_into()?;

        let nodes = {
            if let Some(node_bytes) = r.get_dict_item("nodes") {
                let node_bytes = node_bytes.as_bstr()?;
                bytes_to_nodes4(node_bytes, ID_SIZE)?
            } else {
                Err(Error::new_frame(
                    None,
                    Some(format!("Invalid frame for FindNodeReply, frame: {frame}")),
                ))?
            }
        };

        Ok(FindNodeReply::new(id, nodes, t, v, ip, ro))
    }
}

impl From<FindNodeReply> for Frame {
    fn from(value: FindNodeReply) -> Self {
        let mut rst: BTreeMap<Bytes, Frame> = BTreeMap::new();
        gen_frame_common_field!(rst, value);

        rst.insert("y".into(), "r".into());

        let mut r: BTreeMap<Bytes, Frame> = BTreeMap::new();
        r.insert("id".into(), value.id.get_bytes().into());

        let nodes = merge_node_bytes!(&value.nodes, ID_SIZE);

        r.insert("nodes".into(), nodes.into());

        rst.insert("r".into(), r.into());

        Frame::Map(rst)
    }
}

#[cfg(test)]
mod tests {

    use yiilian_core::data::*;

    use super::*;

    #[test]
    fn test() {
        let id1 = Id::from_bytes(b"node0000000000000001").unwrap();
        let id2 = Id::from_bytes(b"node0000000000000002").unwrap();
        let addr: SocketAddr = "192.168.0.1:1".parse().unwrap();
        let af = FindNodeReply::new(
            "id000000000000000001".try_into().unwrap(),
            vec![Node::new(id1, addr.clone()), Node::new(id2, addr.clone())],
            "t1".into(),
            Some("v1".into()),
            Some("192.168.0.1:1".parse().unwrap()),
            Some(1),
        );

        let rst: Frame = af.clone().into();

        let data = b"d2:ip6:\xc0\xa8\0\x01\0\x011:rd2:id20:id0000000000000000015:nodes52:node0000000000000001\xc0\xa8\0\x01\0\x01node0000000000000002\xc0\xa8\0\x01\0\x01e2:roi1e1:t2:t11:v2:v11:y1:re";

        let data_frame = decode(data.as_slice().into()).unwrap();
        assert_eq!(data_frame, rst);

        let rst: FindNodeReply = data_frame.try_into().unwrap();
        assert_eq!(af, rst);
    }
}
