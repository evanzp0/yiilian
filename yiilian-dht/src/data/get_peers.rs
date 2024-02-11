use std::{collections::BTreeMap, net::SocketAddr};

use bytes::Bytes;

use crate::{common::Id, gen_frame_common_field, transaction::TransactionId};
use yiilian_core::{common::error::Error, data::BencodeData};

use super::{frame::Frame, util::extract_frame_common_field};

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

impl GetPeers {
    pub fn new(
        id: Id,
        info_hash: Id,
        t: TransactionId,
        v: Option<Bytes>,
        ip: Option<SocketAddr>,
        ro: Option<i32>,
    ) -> Self {
        Self {
            id,
            info_hash,
            t,
            v,
            ip,
            ro,
        }
    }
}

impl TryFrom<Frame> for GetPeers {
    type Error = Error;

    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        let (t, v, ip, ro) = extract_frame_common_field(&frame)?;
        if !frame.is_exist_items(&[("y", "q"), ("q", "get_peers")]) {
            return Err(Error::new_frame(
                None,
                Some(format!("Invalid frame for GetPeers, frame: {frame}")),
            ));
        }

        let a = frame.get("a").ok_or(Error::new_frame(
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

        let info_hash: Id = a
            .get_dict_item("info_hash")
            .ok_or(Error::new_frame(
                None,
                Some(format!("Field 'info_hash' not found in frame: {frame}")),
            ))?
            .as_bstr()?
            .to_owned()
            .try_into()?;

        Ok(GetPeers::new(id, info_hash, t, v, ip, ro))
    }
}

impl From<GetPeers> for Frame {
    fn from(value: GetPeers) -> Self {
        let mut rst: BTreeMap<Bytes, BencodeData> = BTreeMap::new();
        gen_frame_common_field!(rst, value);

        rst.insert("y".into(), "q".into());
        rst.insert("q".into(), "get_peers".into());

        let mut a: BTreeMap<Bytes, BencodeData> = BTreeMap::new();
        a.insert("id".into(), value.id.get_bytes().into());
        a.insert("info_hash".into(), value.info_hash.get_bytes().into());

        rst.insert("a".into(), a.into());

        Frame(rst)
    }
}

#[cfg(test)]
mod tests {

    use yiilian_core::data::decode;

    use super::*;

    #[test]
    fn test() {
        let af = GetPeers::new(
            "id000000000000000001".try_into().unwrap(),
            "info0000000000000001".try_into().unwrap(),
            "t1".into(),
            Some("v1".into()),
            Some("127.0.0.1:80".parse().unwrap()),
            Some(1),
        );
        let rst: Frame = af.clone().into();

        let data = b"d1:ad2:id20:id0000000000000000019:info_hash20:info0000000000000001e2:ip6:\x7f\0\0\x01\0P1:q9:get_peers2:roi1e1:t2:t11:v2:v11:y1:qe";
        let data = decode(data.as_slice().into()).unwrap();
        assert_eq!(data, rst.into());

        let rst: GetPeers = Frame::try_from(data).unwrap().try_into().unwrap();
        assert_eq!(af, rst);
    }
}
