use std::{collections::HashMap, net::SocketAddr};

use anyhow::anyhow;
use bytes::Bytes;
use yiilian_core::{YiiLianError, frame::Frame, extract_bencode_field_from_map};

use crate::{
    build_frame_common_field, 
    extract_frame_common_field,
    common::Id,
    transaction::TransactionId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ping {
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

impl Ping {
}

impl TryFrom<Frame> for Ping {
    type Error = YiiLianError;

    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        let (t, v, ip, ro) = extract_frame_common_field!(frame);
        if !frame.verify_items(&[("y", "q"), ("q", "ping")]) {
            return Err(YiiLianError::FrameParse(anyhow!("not valid frame for Ping, frame: {frame}")))
        }
        let a = frame.extract_dict("a")?.as_map()?;

        let id = extract_bencode_field_from_map!(a, "id", frame)?.into();

        Ok(Ping { t, v, ip, ro, id })
    }
}

impl From<&Ping> for Frame {
    fn from(value: &Ping) -> Self {
        let mut rst: HashMap<Bytes, Frame> = HashMap::new();
        build_frame_common_field!(rst, value);

        rst.insert("y".into(), "q".into());
        rst.insert("q".into(), "ping".into());

        let mut a: HashMap<Bytes, Frame> = HashMap::new();
        a.insert("id".into(), value.id.get_bytes().into());
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
        let af = Ping {
            t: "t1".into(),
            v: Some("v1".into()),
            ip: Some(bytes_to_sockaddr(&vec![127, 0, 0, 1, 0,80]).unwrap().into()),
            ro: Some(1),
            id: "id000000000000000001".into(),
        };
        let rst: Frame = (&af).into();

        let data = b"d1:v2:v11:t2:t12:ip6:\x7f\0\0\x01\0\x502:roi1e1:q4:ping1:y1:q1:ad2:id20:id000000000000000001ee";
        let data_frame = decode(data.as_slice().into()).unwrap();
        assert_eq!(data_frame, rst);

        let rst: Ping = data_frame.try_into().unwrap();
        assert_eq!(af, rst);
    }
}
