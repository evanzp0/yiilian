use std::net::SocketAddr;

use bytes::Bytes;
use yiilian_core::{common::error::Error, data::BencodeFrame};

use crate::transaction::TransactionId;

pub(crate) fn extract_frame_common_field(
    frame: &BencodeFrame,
) -> Result<(TransactionId, Option<Bytes>, Option<SocketAddr>, Option<i32>), Error> {
    let t: TransactionId = frame.get_dict_item("t")
        .ok_or(Error::new_frame(None, Some(format!("Field 't' not found in frame: {frame}"))))?
        .as_bstr()?
        .to_owned()
        .into();

    let v: Option<Bytes> = if let Some(val) = frame.get_dict_item("v") {
        if let Ok(val) = val.to_owned().try_into() {
            Some(val)
        } else {
            None
        }
    } else {
        None
    };

    let ro: Option<i32> = if let Some(val) = frame.get_dict_item("ro") {
        if let Ok(val) = val.to_owned().try_into() {
            Some(val)
        } else {
            None
        }
    } else {
        None
    };

    let ip: Option<Bytes> = if let Some(val) = frame.get_dict_item("ip") {
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
            $rst.insert("ro".into(), ro.clone().into());
        }
    };
}
