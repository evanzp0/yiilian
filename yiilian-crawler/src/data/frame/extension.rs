mod ut_metadata;
pub use ut_metadata::*;

use std::{collections::BTreeMap, net::{IpAddr, Ipv4Addr, Ipv6Addr}};

use bytes::Bytes;
use yiilian_core::{common::{error::Error, util::{bytes_to_ip, ip_to_bytes}}, data::{decode, BencodeData, Encode}, map};

/// ExtensionHeader 是扩展握手消息中的 payload
#[derive(Debug)]
pub struct ExtensionHeader {
    pub m: Option<BTreeMap<Bytes, BencodeData>>,
    pub p: Option<u16>,
    pub v: Option<Bytes>,
    pub yourip: Option<IpAddr>,
    pub ipv6: Option<Ipv6Addr>,
    pub ipv4: Option<Ipv4Addr>,
    pub reqq: Option<i32>,
    pub metadata_size: Option<i32>,
}

impl ExtensionHeader {
    pub fn new(
        m: Option<BTreeMap<Bytes, BencodeData>>,
        p: Option<u16>,
        v: Option<Bytes>,
        yourip: Option<IpAddr>,
        ipv6: Option<Ipv6Addr>,
        ipv4: Option<Ipv4Addr>,
        reqq: Option<i32>,
        metadata_size: Option<i32>,
    ) -> Self {
        ExtensionHeader {
            m,
            p,
            v,
            yourip,
            ipv6,
            ipv4,
            reqq,
            metadata_size,
        }
    }

    pub fn new_ut_metadata() -> Self {
        let m: BTreeMap<Bytes, BencodeData> = map! {
            UT_METADATA_NAME.into() => (UT_METADATA_ID as i32).into(),
        };
        let m = Some(m);
        
        ExtensionHeader::new(m, None, None, None, None, None, Some(250), None)
    }

    pub fn get_extension_id(&self, extension_name: &str) -> Option<i32> {
        if let Some(m) = &self.m {
            if let Some(val) = m.get(extension_name.as_bytes()) {
                val.as_int()
                    .map(|msg_id| Some(msg_id))
                    .unwrap_or(None)
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl TryFrom<BencodeData> for ExtensionHeader {
    type Error = Error;

    fn try_from(value: BencodeData) -> Result<Self, Self::Error> {
        if let BencodeData::Map(val) = value {
            let m = if let Some(m) = val.get(&b"m"[..]) {
                Some(m.as_map()?.to_owned())
            } else {
                None
            };
            let p = if let Some(port) = val.get(&b"p"[..]) {
                Some(port.as_int()? as u16)
            } else {
                None
            };
            let v = if let Some(v) = val.get(&b"v"[..]) {
                Some(v.as_bstr()?.to_owned())
            } else {
                None
            };
            let yourip = if let Some(ip) = val.get(&b"yourip"[..]) {
                let ip = ip.as_bstr()?;

                Some(bytes_to_ip(ip)?)
            } else {
                None
            };
            let ipv6 = if let Some(ip) = val.get(&b"ipv6"[..]) {
                let ip = ip.as_bstr()?;
                let ip = bytes_to_ip(ip)?;

                if let IpAddr::V6(ip) = ip{
                    Some(ip)
                } else {
                    None
                }
            } else {
                None
            };
            let ipv4 = if let Some(ip) = val.get(&b"ipv4"[..]) {
                let ip = ip.as_bstr()?;
                let ip = bytes_to_ip(ip)?;

                if let IpAddr::V4(ip) = ip{
                    Some(ip)
                } else {
                    None
                }
            } else {
                None
            };
            let reqq = if let Some(reqq) = val.get(&b"reqq"[..]) {
                Some(reqq.as_int()?)
            } else {
                None
            };
            let metadata_size = if let Some(metadata_size) = val.get(&b"metadata_size"[..]) {
                Some(metadata_size.as_int()?)
            } else {
                None
            };

            let eh = ExtensionHeader::new(
                m,
                p,
                v,
                yourip,
                ipv6,
                ipv4,
                reqq,
                metadata_size,
            );

            Ok(eh)
            
        } else {
            Err(Error::new_frame(None, Some(format!("Can't convert bytes to extension header: {value}"))))?
        }
    }
}

impl TryFrom<Bytes> for ExtensionHeader {
    type Error = Error;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        let value: BencodeData = decode(&value)?;
        let rst: ExtensionHeader = value.try_into()?;

        Ok(rst)
    }
}

impl From<ExtensionHeader> for BencodeData {
    fn from(value: ExtensionHeader) -> Self {
        let mut rst: BTreeMap<Bytes, BencodeData> = BTreeMap::new();

        if let Some(m) = value.m {
            rst.insert("m".into(), m.into());
        }
        
        if let Some(p) = value.p {
            rst.insert("p".into(), (p as i32).into());
        }

        if let Some(v) = value.v {
            rst.insert("v".into(), v.into());
        }

        if let Some(yourip) = value.yourip {
            rst.insert("yourip".into(), (ip_to_bytes(&yourip)).into());
        }

        if let Some(ipv6) = value.ipv6 {
            rst.insert("ipv6".into(), (ipv6.octets()[..]).to_owned().into());
        }

        if let Some(ipv4) = value.ipv4 {
            rst.insert("ipv4".into(), (ipv4.octets()[..]).to_owned().into());
        }

        if let Some(reqq) = value.reqq {
            rst.insert("reqq".into(), reqq.into());
        }
        if let Some(metadata_size) = value.metadata_size {
            rst.insert("metadata_size".into(), metadata_size.into());
        }

        rst.into()
    }
}

impl From<ExtensionHeader> for Bytes {
    fn from(value: ExtensionHeader) -> Self {
        let value : BencodeData = value.into();
        value.encode()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bytes::Bytes;
    use yiilian_core::{data::BencodeData, map};

    use super::{ExtensionHeader, UT_METADATA_NAME};

    #[test]
    fn test_codec() {
        let m: BTreeMap<Bytes, BencodeData> = map! {
            UT_METADATA_NAME.into() => 1.into(),
        };
        let m = Some(m);
        
        let eh = ExtensionHeader::new(m, None, None, None, None, None, Some(250), None);
        let rst: Bytes = eh.into();

        let bytes: Bytes = b"d1:md11:ut_metadatai1ee4:reqqi250ee"[..].into();

        assert_eq!(bytes, rst);

        let eh: ExtensionHeader = bytes.clone().try_into().unwrap();
        let rst: Bytes = eh.into();
        assert_eq!(bytes, rst);
    }

    #[test]
    fn test_get_extension_id() {
        let m: BTreeMap<Bytes, BencodeData> = map! {
            UT_METADATA_NAME.into() => 1.into(),
        };
        let m = Some(m);
        
        let eh = ExtensionHeader::new(m, None, None, None, None, None, Some(250), None);
        let ut_metadata_id = eh.get_extension_id(UT_METADATA_NAME).unwrap();

        assert_eq!(ut_metadata_id, 1);
    }
}