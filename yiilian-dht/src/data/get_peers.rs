use std::{collections::HashMap, net::SocketAddr};

use bytes::Bytes;

use yiilian_core::{common::error::Error, data::BencodeFrame as Frame};
use crate::{common::id::Id, gen_frame_common_field, transaction::TransactionId};

use super::util::extract_frame_common_field;

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

impl TryFrom<Frame> for GetPeers {
    type Error = Error;

    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        let (t, v, ip, ro) = extract_frame_common_field(&frame)?;
        if !frame.verify_items(&[("y", "q"), ("q", "get_peers")]) {
            return Err(Error::new_frame(None, Some(format!("Invalid frame for GetPeers, frame: {frame}"))))
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
            .into();

        let info_hash: Id = a
            .get_dict_item("info_hash")
            .ok_or(Error::new_frame(
                None,
                Some(format!("Field 'info_hash' not found in frame: {frame}")),
            ))?
            .as_bstr()?
            .to_owned()
            .into();

        Ok(GetPeers { t, v, ip, ro, id, info_hash })
    }
}

impl From<&GetPeers> for Frame {
    fn from(value: &GetPeers) -> Self {
        let mut rst: HashMap<Bytes, Frame> = HashMap::new();
        gen_frame_common_field!(rst, value);
        
        rst.insert("y".into(), "q".into());
        rst.insert("q".into(), "get_peers".into());

        let mut a: HashMap<Bytes, Frame> = HashMap::new();
        a.insert("id".into(), value.id.get_bytes().into());
        a.insert("info_hash".into(), value.info_hash.get_bytes().into());

        rst.insert("a".into(), a.into());
        
        Frame::Map(rst)
    }
}

#[cfg(test)]
mod tests {

    use yiilian_core::{common::util::bytes_to_sockaddr, data::decode};

    use super::*;

    #[test]
    fn test() {
        let af = GetPeers {
            t: "t1".into(),
            v: Some("v1".into()),
            ip: Some(bytes_to_sockaddr(&vec![127, 0, 0, 1, 0,80]).unwrap().into()),
            ro: Some(1),
            id: "id000000000000000001".into(),
            info_hash: "info0000000000000001".into(),
        };
        let rst: Frame = (&af).into();
        let data = b"d1:v2:v11:t2:t12:roi1e1:y1:q2:ip6:\x7f\0\0\x01\0\x501:q9:get_peers1:ad2:id20:id0000000000000000019:info_hash20:info0000000000000001ee";
        let data_frame = decode(data.as_slice().into()).unwrap();
        assert_eq!(data_frame, rst);

        let rst: GetPeers = data_frame.try_into().unwrap();
        assert_eq!(af, rst);
    }
}