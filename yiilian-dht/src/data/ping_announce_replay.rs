use std::{collections::BTreeMap, net::SocketAddr};

use bytes::Bytes;
use yiilian_core::{common::error::Error, data::BencodeData};

use crate::{common::Id, gen_frame_common_field, transaction::TransactionId};

use super::{frame::Frame, util::extract_frame_common_field};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PingOrAnnounceReply {
    /// transaction_id
    pub t: TransactionId,

    /// version
    pub v: Option<Bytes>,

    /// 对方看到的我们的外网 IP
    pub ip: Option<SocketAddr>,

    /// readonly
    pub ro: Option<u8>,

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
        ro: Option<u8>,
    ) -> Self {
        Self { id, t, v, ip, ro }
    }
}

impl TryFrom<Frame> for PingOrAnnounceReply {
    type Error = Error;

    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        let (t, v, ip, ro) = extract_frame_common_field(&frame)?;

        if !frame.is_exist_items(&[("y", "r")]) {
            return Err(Error::new_frame(
                None,
                Some(format!(
                    "Invalid frame for PingOrAnnounceReply, frame: {frame}"
                )),
            ));
        }

        let r = frame.get("r").ok_or(Error::new_frame(
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
            .try_into()?;

        Ok(PingOrAnnounceReply::new(id, t, v, ip, ro))
    }
}

impl From<PingOrAnnounceReply> for Frame {
    fn from(value: PingOrAnnounceReply) -> Self {
        let mut rst: BTreeMap<Bytes, BencodeData> = BTreeMap::new();
        gen_frame_common_field!(rst, value);

        rst.insert("y".into(), "r".into());

        let mut r: BTreeMap<Bytes, BencodeData> = BTreeMap::new();
        r.insert("id".into(), value.id.get_bytes().into());

        rst.insert("r".into(), r.into());

        Frame(rst)
    }
}

#[cfg(test)]
mod tests {

    use yiilian_core::data::*;

    use super::*;

    #[test]
    fn test() {
        let af = PingOrAnnounceReply::new(
            "id000000000000000001".try_into().unwrap(),
            "t1".into(),
            Some("v1".into()),
            Some("127.0.0.1:80".parse().unwrap()),
            Some(1),
        );
        let rst: Frame = af.clone().into();

        let data = b"d2:ip6:\x7f\0\0\x01\0P1:rd2:id20:id000000000000000001e2:roi1e1:t2:t11:v2:v11:y1:re";

        let data = decode(data.as_slice().into()).unwrap();
        assert_eq!(data, rst.into());

        let rst: PingOrAnnounceReply = Frame::try_from(data).unwrap().try_into().unwrap();
        assert_eq!(af, rst);
    }
}
