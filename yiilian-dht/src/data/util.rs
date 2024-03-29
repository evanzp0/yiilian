use std::net::SocketAddr;

use bytes::Bytes;
use yiilian_core::common::error::Error;

use crate::transaction::TransactionId;

use super::{body::{Query, Reply}, frame::Frame};

/// 对收到的 reply 消息和 发出的query 消息进行匹配
pub(crate) fn reply_matches_query(query: &Query, reply: &Reply) -> bool {
    match reply {
        Reply::PingOrAnnounce(_) => {
            if let Query::Ping(_) = query {
                return true;
            } else if let Query::AnnouncePeer(_) = query {
                return true;
            }
        }
        Reply::FindNode(_) => {
            if let Query::FindNode(_) = query {
                return true;
            }
        }
        Reply::GetPeers(_) => {
            if let Query::GetPeers(_) = query {
                return true;
            }
        }
    }

    false
}

/// 提取 frame 中的通用字段
pub(crate) fn extract_frame_common_field(
    frame: &Frame,
) -> Result<
    (
        TransactionId,
        Option<Bytes>,
        Option<SocketAddr>,
        Option<u8>,
    ),
    Error,
> {
    let t: TransactionId = frame
        .get("t")
        .ok_or(Error::new_frame(
            None,
            Some(format!("Field 't' not found in frame: {frame}")),
        ))?
        .as_bstr()?
        .to_owned()
        .into();

    let v: Option<Bytes> = if let Some(val) = frame.get("v") {
        if let Ok(val) = val.to_owned().try_into() {
            Some(val)
        } else {
            None
        }
    } else {
        None
    };

    let ro: Option<u8> = if let Some(val) = frame.get("ro") {
        if let Ok(val) = val.as_int() {
            Some(val as u8)
        } else {
            None
        }
    } else {
        None
    };

    let ip: Option<Bytes> = if let Some(val) = frame.get("ip") {
        if let Ok(val) = val.to_owned().try_into() {
            Some(val)
        } else {
            None
        }
    } else {
        None
    };

    let ip = {
        match &ip {
            Some(val) => match yiilian_core::common::util::bytes_to_sockaddr(val) {
                Ok(val) => Some(val),
                Err(_) => None,
            },
            None => None,
        }
    };

    Ok((t, v, ip, ro))
}

/// 生成 frame 中的通用字段
#[macro_export]
macro_rules! gen_frame_common_field {
    ($rst:ident, $value:ident) => {
        $rst.insert("t".into(), $value.t.get_bytes().into());
        if let Some(v) = &$value.v {
            $rst.insert("v".into(), v.clone().into());
        }
        if let Some(ip) = &$value.ip {
            $rst.insert(
                "ip".into(),
                yiilian_core::common::util::sockaddr_to_bytes(&ip).into(),
            );
        }
        if let Some(ro) = $value.ro {
            $rst.insert("ro".into(), (ro as i64).into());
        }
    };
}
