use std::{collections::HashMap, net::SocketAddr};

use bytes::Bytes;
use yiilian_core::{common::error::Error, data::BencodeFrame as Frame};

use crate::{common::id::Id, gen_frame_common_field, transaction::TransactionId};

use super::util::extract_frame_common_field;

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

impl PingOrAnnounceReply {
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

impl TryFrom<Frame> for PingOrAnnounceReply {
    type Error = Error;

    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        let (t, v, ip, ro) = extract_frame_common_field(&frame)?;

        if !frame.verify_items(&[("y", "r")]) {
            return Err(Error::new_frame(
                None,
                Some(format!(
                    "Invalid frame for PingOrAnnounceReply, frame: {frame}"
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

        Ok(PingOrAnnounceReply::new(id, t, v, ip, ro))
    }
}

impl From<PingOrAnnounceReply> for Frame {
    fn from(value: PingOrAnnounceReply) -> Self {
        let mut rst: HashMap<Bytes, Frame> = HashMap::new();
        gen_frame_common_field!(rst, value);

        rst.insert("y".into(), "r".into());

        let mut r: HashMap<Bytes, Frame> = HashMap::new();
        r.insert("id".into(), value.id.get_bytes().into());

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
        let af = PingOrAnnounceReply::new(
            "id000000000000000001".into(),
            "t1".into(),
            Some("v1".into()),
            Some("127.0.0.1:80".parse().unwrap()),
            Some(1),
        );
        let rst: Frame = af.clone().into();

        let data = b"d1:t2:t11:y1:r1:rd2:id20:id000000000000000001e2:roi1e2:ip6:\x7f\0\0\x01\0\x501:v2:v1e";
        let data_frame = decode(data.as_slice().into()).unwrap();
        assert_eq!(data_frame, rst);

        let rst: PingOrAnnounceReply = data_frame.try_into().unwrap();
        assert_eq!(af, rst);
    }
}
