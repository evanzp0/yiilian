use std::{collections::HashMap, net::SocketAddr};

use bytes::Bytes;
use yiilian_core::{common::error::Error, data::BencodeFrame};

use crate::{gen_frame_common_field, common::id::Id, transaction::TransactionId};

use super::util::extract_frame_common_field;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnouncePeer {
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

    /// 资源 info_hash
    pub info_hash: Id,

    /// 0：使用 sock 连接时的 port 作为端口号； 1：使用 port 参数作为端口号
    pub implied_port: Option<u8>,

    /// 端口号
    pub port: u16,

    /// get_peers 请求时从对方获取到的 token
    pub token: Bytes,
}

impl TryFrom<BencodeFrame> for AnnouncePeer {
    type Error = Error;

    fn try_from(frame: BencodeFrame) -> Result<Self, Self::Error> {
        let (t, v, ip, ro) = extract_frame_common_field(&frame)?;
        if !frame.verify_items(&[("y", "q"), ("q", "announce_peer")]) {
            Err(Error::new_frame(
                None,
                Some(format!("not valid frame for AnnouncePeer, frame: {frame}")),
            ))?
        }

        let a = frame.get_dict_item("a")
            .ok_or(Error::new_frame(None, Some(format!("Field 'a' not found in frame: {frame}"))))?;
        let id: Id = a.get_dict_item("id")
            .ok_or(Error::new_frame(None, Some(format!("Field 'id' not found in frame: {frame}"))))?
            .as_bstr()?
            .to_owned()
            .into();
        
        let info_hash: Id = a.get_dict_item("info_hash")
            .ok_or(Error::new_frame(None, Some(format!("Field 'info_hash' not found in frame: {frame}"))))?
            .as_bstr()?
            .to_owned()
            .into();

        let implied_port = {
            if let Some(implied_port) = a.get_dict_item("implied_port") {
                Some(implied_port.as_int()? as u8)
            } else {
                None
            }
        };

        let port: u16 = a.get_dict_item("port")
            .ok_or(Error::new_frame(None, Some(format!("Field 'port' not found in frame: {frame}"))))?
            .as_int()? as u16;

        let token: Bytes = a.get_dict_item("token")
            .ok_or(Error::new_frame(None, Some(format!("Field 'token' not found in frame: {frame}"))))?
            .to_owned()
            .try_into()?;

        Ok(AnnouncePeer {
            t,
            v,
            ip,
            ro,
            id,
            info_hash,
            implied_port,
            port,
            token,
        })
    }
}

impl From<&AnnouncePeer> for BencodeFrame {
    fn from(value: &AnnouncePeer) -> Self {
        let mut rst: HashMap<Bytes, BencodeFrame> = HashMap::new();
        gen_frame_common_field!(rst, value);

        rst.insert("y".into(), "q".into());
        rst.insert("q".into(), "announce_peer".into());

        let mut a: HashMap<Bytes, BencodeFrame> = HashMap::new();
        a.insert("id".into(), value.id.get_bytes().into());
        a.insert("info_hash".into(), value.info_hash.get_bytes().into());
        if let Some(implied_port) = value.implied_port {
            a.insert("implied_port".into(), (implied_port as i32).into());
        }
        a.insert("port".into(), (value.port as i32).into());
        a.insert("token".into(), value.token.clone().into());

        rst.insert("a".into(), a.into());

        BencodeFrame::Map(rst)
    }
}

#[cfg(test)]
mod tests {

    use yiilian_core::{common::util::bytes_to_sockaddr, data::decode};

    use super::*;

    #[test]
    fn test() {
        let af = AnnouncePeer {
            t: "t1".into(),
            v: Some("v1".into()),
            ip: Some(
                bytes_to_sockaddr(&vec![127, 0, 0, 1, 0, 80])
                    .unwrap()
                    .into(),
            ),
            ro: Some(1),
            id: "id000000000000000001".into(),
            info_hash: "info0000000000000001".into(),
            implied_port: Some(1),
            port: 80,
            token: "01".into(),
        };
        let rst: BencodeFrame = (&af).into();

        let data = b"d1:t2:t11:v2:v12:ip6:\x7f\0\0\x01\0\x502:roi1e1:q13:announce_peer1:ad2:id20:id0000000000000000019:info_hash20:info00000000000000014:porti80e5:token2:0112:implied_porti1ee1:y1:qe";
        let data_frame = decode(data.as_slice().into()).unwrap();
        assert_eq!(data_frame, rst);

        let rst: AnnouncePeer = data_frame.try_into().unwrap();
        assert_eq!(af, rst);
    }
}
