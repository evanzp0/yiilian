use std::{collections::HashMap, net::SocketAddr};

use bytes::Bytes;
use yiilian_core::{common::error::Error, data::BencodeFrame};

use crate::{gen_frame_common_field, transaction::TransactionId};

use super::util::extract_frame_common_field;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RError {
    /// transaction_id
    pub t: TransactionId,

    /// version
    pub v: Option<Bytes>,

    /// 对方看到的我们的外网 IP
    pub ip: Option<SocketAddr>,

    /// readonly
    pub ro: Option<i32>,

    // ----------------------------

    /// error code & message
    pub e: (i32, Bytes)
}

impl RError {
}

impl TryFrom<BencodeFrame> for RError {
    type Error = Error;

    fn try_from(frame: BencodeFrame) -> Result<Self, Self::Error> {
        let (t, v, ip, ro) = extract_frame_common_field(&frame)?;
        if !frame.verify_items(&[("y", "e")]) {
            return Err(Error::new_frame(None, Some(format!("Field 'y' and 'e' not found in frame: {frame}"))))
        }
        let e = frame.get_dict_item("e")
            .ok_or(Error::new_frame(None, Some(format!("Field 'e' not found in frame: {frame}"))))?
            .as_list()?;
        let code = if let Some(code) = e.get(0) {
            code.as_int()?
        } else {
            Err(Error::new_frame(None, Some(format!("Not found valid code item in 'e' dict frame: {frame}"))))?
        };
        let msg = if let Some(msg) = e.get(1) {
            msg.as_bstr()?.clone()
        } else {
            Err(Error::new_frame(None,  Some(format!("Not found valid msg item in 'e' dict frame: {frame}"))))?
        };
        let e = (code, msg);

        Ok(RError { t, v, ip, ro, e })
    }
}

impl From<&RError> for BencodeFrame {
    fn from(value: &RError) -> Self {
        let mut rst: HashMap<Bytes, BencodeFrame> = HashMap::new();
        gen_frame_common_field!(rst, value);

        rst.insert("y".into(), "e".into());

        let mut e: Vec<BencodeFrame> = Vec::new();
        e.push(BencodeFrame::Int(value.e.0));
        e.push(BencodeFrame::Str(value.e.1.clone()));
        
        rst.insert("e".into(), e.into());

        BencodeFrame::Map(rst)
    }
}

#[cfg(test)]
mod tests {

    use yiilian_core::{common::util::bytes_to_sockaddr, data::decode};

    use super::*;

    #[test]
    fn test() {
        let af = RError {
            t: "t1".into(),
            v: Some("v1".into()),
            ip: Some(bytes_to_sockaddr(&vec![127, 0, 0, 1, 0,80]).unwrap().into()),
            ro: Some(1),
            e: (200, "a_error".into()),
        };
        let rst: BencodeFrame = (&af).into();

        let data = b"d1:v2:v11:t2:t12:ip6:\x7f\0\0\x01\0\x502:roi1e1:y1:e1:eli200e7:a_erroree";
        // let data = b"d1:eli202e12:Server Errore1:t2:&]1:y1:ee";
        // let data = b"d1:eli203e17:No transaction IDe1:v4:lt\r`1:y1:ee";
        let data_frame = decode(data.as_slice().into()).unwrap();
        // println!("frame: {:#?}", data_frame);
        assert_eq!(data_frame, rst);

        let rst: RError = data_frame.try_into().unwrap();
        assert_eq!(af, rst);
    }
}
