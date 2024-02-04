use std::{collections::HashMap, net::SocketAddr};

use bytes::Bytes;
use yiilian_core::{common::error::Error, data::BencodeFrame as Frame};

use crate::{common::Id, gen_frame_common_field, transaction::TransactionId};

use super::util::extract_frame_common_field;

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
    pub fn new(
        id: Id,
        t: TransactionId,
        v: Option<Bytes>,
        ip: Option<SocketAddr>,
        ro: Option<i32>,
    ) -> Self {
        Self { id, t, v, ip, ro }
    }
}

impl TryFrom<Frame> for Ping {
    type Error = Error;

    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        let (t, v, ip, ro) = extract_frame_common_field(&frame)?;
        if !frame.verify_items(&[("y", "q"), ("q", "ping")]) {
            return Err(Error::new_frame(None, Some(format!("Invalid frame for Ping, frame: {frame}"))))
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

        Ok(Ping::new(id, t, v, ip, ro))
    }
}

impl From<Ping> for Frame {
    fn from(value: Ping) -> Self {
        let mut rst: HashMap<Bytes, Frame> = HashMap::new();
        gen_frame_common_field!(rst, value);

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

    use yiilian_core::data::decode;

    use super::*;

    #[test]
    fn test() {
        let af = Ping::new(
            "id000000000000000001".try_into().unwrap(),
            "t1".into(),
            Some("v1".into()),
            Some("127.0.0.1:80".parse().unwrap()),
            Some(1),
        );
        let rst: Frame = af.clone().into();

        let data = b"d1:v2:v11:t2:t12:ip6:\x7f\0\0\x01\0\x502:roi1e1:q4:ping1:y1:q1:ad2:id20:id000000000000000001ee";
        let data_frame = decode(data.as_slice().into()).unwrap();
        assert_eq!(data_frame, rst);

        let rst: Ping = data_frame.try_into().unwrap();
        assert_eq!(af, rst);
    }
}
