use std::{collections::HashMap, net::SocketAddr};

use bytes::Bytes;
use yiilian_core::{
    common::{
        error::Error,
        util::{bytes_to_sockaddr, sockaddr_to_bytes},
    },
    data::BencodeFrame as Frame,
};

use crate::{
    common::{
        id::{Id, ID_SIZE},
        util::bytes_to_nodes4,
    },
    gen_frame_common_field, merge_node_bytes,
    routing_table::Node,
    transaction::TransactionId,
};

use super::util::extract_frame_common_field;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetPeersReply {
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

    /// 对方在 announce_peer 请求中需要回传该 token
    pub token: Bytes,

    /// reply nodes
    pub nodes: Vec<Node>,

    /// reply values
    pub values: Vec<SocketAddr>,
}

impl GetPeersReply {
    pub fn new(
        id: Id,
        token: Bytes,
        nodes: Vec<Node>,
        values: Vec<SocketAddr>,
        t: TransactionId,
        v: Option<Bytes>,
        ip: Option<SocketAddr>,
        ro: Option<i32>,
    ) -> Self {
        Self {
            id,
            token,
            nodes,
            values,
            t,
            v,
            ip,
            ro,
        }
    }
}

impl TryFrom<Frame> for GetPeersReply {
    type Error = Error;

    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        let (t, v, ip, ro) = extract_frame_common_field(&frame)?;
        if !frame.verify_items(&[("y", "r")]) {
            return Err(Error::new_frame(
                None,
                Some(format!(
                    "Invalid frame for GetPeersFeedback, frame: {frame}"
                )),
            ));
        }

        let r = frame.get_dict_item("r").ok_or(Error::new_frame(
            None,
            Some(format!("Field 'r' not found in frame: {frame}")),
        ))?;

        let id: Id = r
            .get_dict_item("id")
            .ok_or(Error::new_frame(
                None,
                Some(format!("Field 'id' not found in frame: {frame}")),
            ))?
            .as_bstr()?
            .to_owned()
            .into();

        let token: Bytes = r
            .get_dict_item("token")
            .ok_or(Error::new_frame(
                None,
                Some(format!("Field 'token' not found in frame: {frame}")),
            ))?
            .to_owned()
            .try_into()?;

        let nodes = if let Some(node_bytes) = r.get_dict_item("nodes") {
            let node_bytes = node_bytes.as_bstr()?;
            let nodes = bytes_to_nodes4(node_bytes, ID_SIZE)?;
            nodes
        } else {
            vec![]
        };

        let values = if let Some(value_bytes) = r.get_dict_item("values") {
            let vb = value_bytes.as_list()?;
            let mut values = vec![];
            for item in vb {
                let addr = bytes_to_sockaddr(item.as_bstr()?)?;
                values.push(addr);
            }
            values
        } else {
            vec![]
        };

        Ok(GetPeersReply::new(id, token, nodes, values, t, v, ip, ro))
    }
}

impl From<GetPeersReply> for Frame {
    fn from(value: GetPeersReply) -> Self {
        let mut rst: HashMap<Bytes, Frame> = HashMap::new();
        gen_frame_common_field!(rst, value);

        rst.insert("y".into(), "r".into());

        let mut r: HashMap<Bytes, Frame> = HashMap::new();
        r.insert("id".into(), value.id.get_bytes().into());
        r.insert("token".into(), value.token.clone().into());

        r.insert(
            "nodes".into(),
            merge_node_bytes!(&value.nodes, ID_SIZE).into(),
        );
        // r.insert("values".into(), merge_socket_addr_bytes!(&value.values).into());
        let mut values = vec![];
        for item in &value.values {
            let compact_ip_port = sockaddr_to_bytes(&item);
            values.push(Frame::Str(compact_ip_port.into()));
        }
        r.insert("values".into(), values.into());

        rst.insert("r".into(), r.into());

        Frame::Map(rst)
    }
}

#[cfg(test)]
mod tests {
    use yiilian_core::data::decode;

    use super::*;

    #[test]
    fn test() {
        let addr: SocketAddr = "192.168.0.1:80".parse().unwrap();
        let af = GetPeersReply::new(
            "id000000000000000001".into(),
            "token01".into(),
            vec![],
            vec![addr.clone(), addr.clone()],
            "t1".into(),
            Some("v1".into()),
            Some("127.0.0.1:80".parse().unwrap()),
            Some(1),
        );
        let rst: Frame = af.clone().into();

        let data = b"d2:ip6:\x7f\0\0\x01\0P2:roi1e1:t2:t11:rd2:id20:id0000000000000000015:token7:token016:valuesl6:\xc0\xa8\0\x01\0P6:\xc0\xa8\0\x01\0Pe5:nodes0:e1:y1:r1:v2:v1e";
        let data_frame = decode(data).unwrap();
        assert_eq!(data_frame, rst);

        let rst: GetPeersReply = data_frame.try_into().unwrap();
        assert_eq!(af, rst);
    }

    #[test]
    fn test_decode() {
        let data = b"d2:ip6:e];\x99\x11\xae1:rd2:id20:d\x8d\x89W\xe3\xa9D\x1cF\xa4'7\xf0\xfbf\xf6\x81\x1d\xbd\xd95:nodes208:dD0\xf5x-E\x84\xa1l\x9a\x90\x9dU\x804\xeb\0\x03`t\xe9\xd6\xad4\xa0d[\xa0\x86*\xeb\x8c*@\xdeR?\r\x93!D\xe4\x9c\x95z\x01\xa0\xd7\x0e[(de\x9fM36\xf7\xa6Y\xdb\x83\xd7o\xe9\xed\x0b\x861\xf5h\xc9)\xce\xfa\x958d~D\x1aO\x01-O\xa4i\0\xf6\x98\x13\xaa<3<\x87\xca\xd2\xc3\xe0\xffK\x16d\x01\xe1\xf6\xf9\xd9=I\x85L\xca\xd5h\x8d\xdbuC\xce\xfd1R+\xf7\x0e\xe63d\x19\xae\xfeV*\x07\x91\xfcTu\xc6(\xaf\0\x8d\xd6\xdd\x15\xb5t\x11f\x82^\x1dd(\xd6nZ@\x1c\xf7A\x8cK\x97W\x8b\xfc\x12\xfc\xc5\x1f\xa5ZOA\x07\x1a\xe1d:\xc0\xb9\xfb\x83\xdb<\x1a\xdf;Pd\xb8\xc7aGE\xbe\xc8Y\x85\xa0g?T5:token20:\xb3\xfa\xbaA\xc0~b\x08\x8cz\xa6\xa1\xdf\x87\x9aP\xc9\x88K\xd56:valuesl6:\xa8w$\xaeV\xcfee1:t2:\x94\x881:v4:UT\xb7`1:y1:re";
        let data_frame: Frame = decode(data).unwrap();
        // println!("{:#?}", data_frame);

        let _rst: GetPeersReply = data_frame.try_into().unwrap();
        // println!("{:#?}", rst);
    }
}
