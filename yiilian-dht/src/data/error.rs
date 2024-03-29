use std::{collections::BTreeMap, net::SocketAddr};

use bytes::Bytes;
use yiilian_core::{common::error::Error, data::BencodeData};

use crate::{gen_frame_common_field, transaction::TransactionId};

use super::{frame::Frame, util::extract_frame_common_field};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RError {
    /// transaction_id
    pub t: TransactionId,

    /// version
    pub v: Option<Bytes>,

    /// 对方看到的我们的外网 IP
    pub ip: Option<SocketAddr>,

    /// readonly
    pub ro: Option<u8>,

    // ----------------------------
    /// error code & message
    pub e: (i64, Bytes),
}

impl RError {
    pub fn new(
        code: i64,
        message: Bytes,
        t: TransactionId,
        v: Option<Bytes>,
        ip: Option<SocketAddr>,
        ro: Option<u8>,
    ) -> Self {
        let e = (code, message);
        Self { e, t, v, ip, ro }
    }
}

impl TryFrom<Frame> for RError {
    type Error = Error;

    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        let (t, v, ip, ro) = extract_frame_common_field(&frame)?;
        if !frame.is_exist_items(&[("y", "e")]) {
            return Err(Error::new_frame(
                None,
                Some(format!("Invalid frame for Error, frame: {frame}")),
            ));
        }
        let e = frame
            .get("e")
            .ok_or(Error::new_frame(
                None,
                Some(format!("Field 'e' not found in frame: {frame}")),
            ))?
            .as_list()?;
        let code = if let Some(code) = e.get(0) {
            code.as_int()?
        } else {
            Err(Error::new_frame(
                None,
                Some(format!(
                    "Not found valid code item in 'e' dict frame: {frame}"
                )),
            ))?
        };
        let message = if let Some(msg) = e.get(1) {
            msg.as_bstr()?.clone()
        } else {
            Err(Error::new_frame(
                None,
                Some(format!(
                    "Not found valid msg item in 'e' dict frame: {frame}"
                )),
            ))?
        };

        Ok(RError::new(code, message, t, v, ip, ro))
    }
}

impl From<RError> for Frame {
    fn from(value: RError) -> Self {
        let mut rst: BTreeMap<Bytes, BencodeData> = BTreeMap::new();
        gen_frame_common_field!(rst, value);

        rst.insert("y".into(), "e".into());

        let mut e: Vec<BencodeData> = Vec::new();
        e.push(BencodeData::Int(value.e.0));
        e.push(BencodeData::Str(value.e.1.clone()));

        rst.insert("e".into(), e.into());

        Frame(rst)
    }
}

#[cfg(test)]
mod tests {

    use yiilian_core::data::*;

    use super::*;

    #[test]
    fn test() {
        let af = RError::new(
            200,
            "a_error".into(),
            "t1".into(),
            Some("v1".into()),
            Some("127.0.0.1:80".parse().unwrap()),
            Some(1),
        );

        let rst: Frame = af.clone().into();
        let data = b"d1:eli200e7:a_errore2:ip6:\x7f\0\0\x01\0P2:roi1e1:t2:t11:v2:v11:y1:ee";
        // let data = b"d1:eli202e12:Server Errore1:t2:&]1:y1:ee";
        // let data = b"d1:eli203e17:No transaction IDe1:v4:lt\r`1:y1:ee";
        let data = decode(data.as_slice().into()).unwrap();
        // println!("frame: {:#?}", data_frame);
        assert_eq!(data, rst.into());

        let frame = Frame::try_from(data).unwrap();
        let rst: RError = frame.try_into().unwrap();
        assert_eq!(af, rst);
    }
}
