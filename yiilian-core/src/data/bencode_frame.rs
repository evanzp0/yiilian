use std::{collections::HashMap, fmt::Display};

use bytes::Bytes;
use crate::common::{util::atoi, error::Error};

/// Frame 的帧
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum BencodeFrame {
    Str(Bytes),
    Int(i32),
    List(Vec<BencodeFrame>),
    Map(HashMap<Bytes, BencodeFrame>),
}

impl BencodeFrame {
    /// 从 Bytes 中解析出 Frame
    pub fn parse(data: &[u8]) -> Result<BencodeFrame, Error> {
        decode(data)
    }

    pub fn as_bstr(&self) -> Result<&Bytes, Error> {
        if let BencodeFrame::Str(v) = self {
            Ok(v)
        } else {
            Err(Error::new_frame(None, Some("error type for Frame's as_bstr()".to_owned())))?
        }
    }

    pub fn as_int(&self) -> Result<i32, Error> {
        if let BencodeFrame::Int(v) = self {
            Ok(v.to_owned())
        } else {
            Err(Error::new_frame(None, Some("error type for Frame's as_int()".to_owned())))?
        }
    }

    pub fn as_map(&self) -> Result<&HashMap<Bytes, BencodeFrame>, Error> {
        if let BencodeFrame::Map(v) = self {
            Ok(v)
        } else {
            Err(Error::new_frame(None, Some("error type for Frame's as_map()".to_owned())))?
        }
    }

    pub fn as_list(&self) -> Result<&[BencodeFrame], Error> {
        if let BencodeFrame::List(v) = self {
            Ok(v)
        } else {
            Err(Error::new_frame(None, Some("error type for Frame's as_list()".to_owned())))?
        }
    }

    pub fn verify_items(&self, items: &[(&str, &str)]) -> bool {
        if let BencodeFrame::Map(m) = self {
            for (key, val) in items {
                if let Some(v) = m.get(key.as_bytes()) {
                    match v.as_bstr() {
                        Ok(v) =>  {
                            if v != *val {
                                return false;
                            }
                        },
                        Err(_) => return false,
                    }
                } else {
                    return false;
                }
            }
            return true;
        }

        false
    }

    pub fn has_key(&self, key: &'static str) -> bool {
        if let BencodeFrame::Map(m) = self {
            match m.get(&Bytes::from(key)) {
                Some(_) => true,
                None => false,
            }
        } else {
            false
        }
    }

    pub fn extract_dict(&self, key: &'static str) -> Result<&BencodeFrame, Error> {
        if let BencodeFrame::Map(m) = self {
            let rst = m.get(&Bytes::from(key))
                .ok_or(
                    Error::new_frame(None, Some(format!("Can't find '{}' in the frame", key)))
                )?;

            Ok(rst)
        } else {
            Err(
                Error::new_frame(None, Some(format!("extract_dict: not a invalid frame: {}", self.to_string())))
            )
        }
    }

    fn to_string(&self) -> String {
        match self {
            BencodeFrame::Str(s) => {
                format!("{:?}", s)
            },
            BencodeFrame::Int(i) => {
                format!("{}", i)
            },
            BencodeFrame::List(l) => {
                let mut rst = format!("[ ");
                for item in l {
                    rst += &format!("{}, ", item.to_string());
                }
                rst.remove(rst.len() - 2);
                rst += &format!("]");

                rst
            },
            BencodeFrame::Map(m) => {
                let mut rst = format!("{{ ");
                for (key, val) in m {
                    rst += &format!("{:?}: {}, ", key, val.to_string());
                }
                rst.remove(rst.len() - 2);
                rst += &format!("}}");

                rst
            }
        }
    }
}

impl Display for BencodeFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl From<&'static str> for BencodeFrame {
    fn from(value: &'static str) -> Self {
        BencodeFrame::Str(Bytes::from(value))
    }
}

impl From<&'static [u8]> for BencodeFrame {
    fn from(value: &'static [u8]) -> Self {
        BencodeFrame::Str(value.into())
    }
}

impl From<Bytes> for BencodeFrame {
    fn from(value: Bytes) -> Self {
        BencodeFrame::Str(value)
    }
}

impl From<Vec<u8>> for BencodeFrame {
    fn from(value: Vec<u8>) -> Self {
        BencodeFrame::Str(value.into())
    }
}

impl From<String> for BencodeFrame {
    fn from(value: String) -> Self {
        BencodeFrame::Str(value.into())
    }
}

impl From<i32> for BencodeFrame {
    fn from(value: i32) -> Self {
        BencodeFrame::Int(value)
    }
}

impl From<Vec<BencodeFrame>> for BencodeFrame {
    fn from(value: Vec<BencodeFrame>) -> Self {
        BencodeFrame::List(value)
    }
}

impl From<HashMap<Bytes, BencodeFrame>> for BencodeFrame {
    fn from(value: HashMap<Bytes, BencodeFrame>) -> Self {
        BencodeFrame::Map(value)
    }
}

impl From<HashMap<String, BencodeFrame>> for BencodeFrame {
    fn from(value: HashMap<String, BencodeFrame>) -> Self {
        let mut rst: HashMap<Bytes, BencodeFrame> = HashMap::new();

        for (key, val) in value {
            rst.insert(key.into(), val);
        }

        BencodeFrame::Map(rst)
    }
}

/// find returns the index of first target in data starting from `start`.
pub fn find(data: &[u8], start: usize, target: u8) -> Option<usize> {
    let index = data[start..]
        .iter()
        .position(|&c| c == target)
        .map(|v| start + v);

    index
}

/// DecodeString decodes a string in the data. It returns a
/// Result<(decoded result, the end position), error>.
pub fn decode_string(data: &[u8], start: usize) -> Result<(BencodeFrame, usize), Error> {

    if start >= data.len() || data[start] < b'0' || data[start] > b'9' {
        return Err(
            Error::new_frame(None, Some("invalid string bencode".to_owned()))
        );
    }

    let idx = find(data, start, b':');
    if idx == None {
        return Err(
            Error::new_frame(None, Some("':' not found when decode string".to_owned())));
    }

    let idx = idx.unwrap();
    let length = atoi(&data[start..idx])?;
    let index = idx + 1 + (length as usize);

    if index > data.len() || index < idx + 1 {
        return Err(
            Error::new_frame(None, Some("':' out of range".to_owned())));
    }

    let rst = data[(idx + 1)..index].to_vec();

    Ok((rst.into(), index))
}

pub fn decode_int(data: &[u8], start: usize) -> Result<(BencodeFrame, usize), Error> {
    if start >= data.len() || data[start] != b'i' {
        return Err(
            Error::new_frame(None, Some("invalid int bencode".to_owned())));
    }

    let start = start + 1;
    let idx = find(data, start, b'e');
    if idx == None {
        return Err(
            Error::new_frame(None, Some("'e' not found when decode string".to_owned())));
    }

    let idx = idx.unwrap();
    let s = String::from_utf8_lossy(&data[start..idx]);
    let rst = if let Ok(v) = s.parse::<i32>() {
        v
    } else {
        return Err(
            Error::new_frame(None, Some("can't pasrse to i32".to_owned())));
    };

    let index = idx + 1;

    Ok((rst.into(), index))
}

/// decodeItem decodes an item of dict or list.
pub fn decode_item(data: &[u8], start: usize) -> Result<(BencodeFrame, usize), Error> {
    let decode_func = [decode_string, decode_int, decode_list, decode_dict];

    for func in decode_func {
        let rst = func(data, start);
        if let Ok(_) = rst {
            return rst;
        }
    }

    Err(
        Error::new_frame(None, Some("invalid bencode when decode item".to_owned())))
}

/// DecodeList decodes a list value.
pub fn decode_list(data: &[u8], start: usize) -> Result<(BencodeFrame, usize), Error> {
    if start >= data.len() || data[start] != b'l' {
        return Err(
            Error::new_frame(None, Some("invalid list bencode".to_owned()))
        );
    }

    let mut rst: Vec<BencodeFrame> = Vec::new();
    let mut index = start + 1;

    while index < data.len() {
        if data[index] == b'e' {
            break;
        }

        let (item, idx) = decode_item(data, index)?;
        rst.push(item);

        index = idx;
    }

    if index == data.len() {
        return Err(
            Error::new_frame(None, Some("'e' not found when decode list".to_owned())));
    }

    index += 1;

    Ok((rst.into(), index))
}

/// DecodeDict decodes a map value.
pub fn decode_dict(data: &[u8], start: usize) -> Result<(BencodeFrame, usize), Error> {
    if start >= data.len() || data[start] != b'd' {
        return Err(
            Error::new_frame(None, Some("invalid dict bencode".to_owned())));
    }

    let mut rst: HashMap<Bytes, BencodeFrame> = HashMap::new();
    let mut index = start + 1;

    while index < data.len() {
        if data[index] == b'e' {
            break;
        }

        if !data[index].is_ascii_digit() {
            return Err(
                Error::new_frame(None, Some("invalid dict bencode".to_owned())));
        }

        let (b_key, idx) = decode_string(data, index)?;
        let key = if let BencodeFrame::Str(k) = b_key {
            k
        } else {
            Default::default()
        };

        if idx >= data.len() {
            return Err(
                Error::new_frame(None, Some("out of range when decode dict".to_owned())));
        }

        let (item, idx) = decode_item(data, idx)?;
        rst.insert(key, item);

        index = idx;
    }

    if index == data.len() {
        return Err(
            Error::new_frame(None, Some("'e' not found when decode dict".to_owned())));
    }

    index += 1;

    Ok((rst.into(), index))
}

/// Decode decodes a bencoded string to string, int, list or map.
pub fn decode(data: &[u8]) -> Result<BencodeFrame, Error> {
    let (rst, _) = decode_item(data, 0)?;
    Ok(rst)
}

pub trait Encode {
    fn encode(&self) -> Bytes;
}

impl Encode for i32 {
    fn encode(&self) -> Bytes {
        ("i".to_string() + &self.to_string() + "e").into()
    }
}

impl Encode for String {
    fn encode(&self) -> Bytes {
        Bytes::from(Vec::from(self.as_bytes())).encode()
    }
}

impl Encode for &'static str {
    fn encode(&self) -> Bytes {
        Bytes::from(*self).encode()
    }
}

impl Encode for Bytes {
    fn encode(&self) -> Bytes {
        let mut rst: Vec<u8> = vec![];
        rst.extend(self.len().to_string().as_bytes());
        rst.push(b':');
        rst.extend(self);

        rst.into()
    }
}

impl Encode for &'static [u8] {
    fn encode(&self) -> Bytes {
        Bytes::from(*self).encode()
    }
}

impl Encode for Vec<u8> {
    fn encode(&self) -> Bytes {
        Bytes::from(self.clone()).encode()
    }
}

impl<T> Encode for Vec<T>
where
    T: Encode,
{
    fn encode(&self) -> Bytes {
        let mut rst = vec![b'l'];
        for item in self {
            rst.extend(item.encode());
        }
        rst.push(b'e');

        rst.into()
    }
}

impl<K, V> Encode for HashMap<K, V>
where
    K: Encode,
    V: Encode,
{
    fn encode(&self) -> Bytes {
        let mut rst = vec![b'd'];
        for (key, value) in self {
            rst.extend(key.encode());
            rst.extend(value.encode());
        }
        rst.push(b'e');

        rst.into()
    }
}

impl Encode for BencodeFrame
{
    fn encode(&self) -> Bytes {
        match self {
            BencodeFrame::Str(v) => v.encode(),
            BencodeFrame::Int(v) => v.encode(),
            BencodeFrame::List(v) => v.encode(),
            BencodeFrame::Map(v) => v.encode(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::hashmap;

    use super::*;
    use super::BencodeFrame::*;

    #[test]
    fn test_find() {
        let data = "1:c2:ab3de".as_bytes();
        assert_eq!(Some(1), find(data, 0, b':'));
        assert_eq!(Some(4), find(data, 2, b':'));
        assert_eq!(None, find(data, 0, b'z'));
    }

    #[test]
    fn test_decode_string() {
        let data = "21:c2:ab3dessssssssssst".as_bytes();
        assert_eq!(("ab".into(), 8), decode_string(data, 4).unwrap());
        assert!(decode_string(data, 2).unwrap_err().to_string().find("invalid string bencode").is_some());
        assert!(decode_string(data, 8).unwrap_err().to_string().find("':' not found when decode string").is_some());
        assert!(decode_string(data, 0).unwrap_err().to_string().find("out of range").is_some());
    }

    #[test]
    fn test_decode_int() {
        let data = "2:abi123ei1".as_bytes();
        assert_eq!((123.into(), 9), decode_int(data, 4).unwrap());
        assert!(decode_int(data, 0).unwrap_err().to_string().find("invalid int bencode").is_some());
        assert!(decode_int(data, 9).unwrap_err().to_string().find("'e' not found when decode string").is_some());
    }

    #[test]
    fn test_decode_item() {
        let data = "2:abi123el2:ab3:xyze".as_bytes();
        
        assert_eq!(("ab".into(), 4), decode_item(data, 0).unwrap());
        assert_eq!((123.into(), 9), decode_item(data, 4).unwrap());
        assert_eq!(
            (vec![Str("ab".into()), Str("xyz".into())].into(), 20),
            decode_item(data, 9).unwrap()
        );
    }

    #[test]
    fn test_decode_list() {
        let data = "l2:ab3:xyze".as_bytes();
        assert_eq!(
            (vec![Str("ab".into()), Str("xyz".into())].into(), 11),
            decode_list(data, 0).unwrap()
        );

        let data = "li12ei345ee".as_bytes();

        assert_eq!(
           (vec![Int(12), Int(345)].into(), 11),
            decode_list(data, 0).unwrap()
        );
    }

    #[test]
    fn test_decode_dict() {
        let data = "d2:abi12ee".as_bytes();
        let mut rst: HashMap<String, BencodeFrame> = HashMap::new();
        rst.insert("ab".into(), 12.into());

        assert_eq!((rst.into(), 10), decode_dict(data, 0).unwrap());

        let data = "d2:ab3:xyze".as_bytes();
        let mut rst: HashMap<Bytes, BencodeFrame> = HashMap::new();
        rst.insert("ab".into(), "xyz".into());

        assert_eq!((rst.into(), 11), decode_dict(data, 0).unwrap());
    }

    #[test]
    fn test_decode() {
        let data = "d2:abl3:xyzi123ee3:aaa2:bbe";
        let data_decoded: HashMap<Bytes, BencodeFrame> = hashmap! {
            "ab".into() => vec![BencodeFrame::from("xyz"), 123.into()].into(),
            "aaa".into() => "bb".into()
        };

        assert_eq!(Map(data_decoded), decode(data.as_bytes()).unwrap());
    }

    #[test]
    fn test_encode() {
        assert_eq!("i-123e", (-123).encode());
        assert_eq!("i123e", 123.encode());
        assert_eq!("3:abc", "abc".to_string().encode());
        assert_eq!("3:abc", "abc".encode());
        assert_eq!("l3:abc2:dee", vec!["abc", "de"].encode());
        assert_eq!("li1ei22ee", vec![1, 22].encode());

        let m: HashMap<Bytes, String> = hashmap!{
            "k1".into() => "aa".into(),
        };
        assert_eq!("d2:k12:aae", m.encode());

        assert_eq!("l2:aai123ee", List(vec![Str("aa".into()), Int(123)]).encode())
    }

    #[test]
    fn test_x() {
        let data = b"6:\xc0\xa8\0\x01\0\x01";
        let rst = decode(data).unwrap();

        let frame = BencodeFrame::Str(b"\xc0\xa8\0\x01\0\x01".as_ref().into());

        assert_eq!(frame, rst);
    }

    #[test]
    fn test_online_bug() {
        // data 长度为 2048，但是 nodes 字节：9828，所以异常
        let data = b"d1:rd2:id20:\xbb\xf3TAX\xaf]\xa2(\x15Y\x97\x14\xde\xc5\xde\xbf\xbc~O5:nodes9828:\x9fz\x1b\xa8\x80gx{\xdf\xdb\xe1\xa9\x11\x02\x96\xe3\x05\x03w\x02\xad\xf9,\xb9\xad\x99\x99t\x89\xa2\xae\x90\x10\xef\x04\x99\xedp\x8ex\xfe,d\xcf\x82\xa9\xaco&\x80\xb5\xbd\x98%\xb2\x9e\x93g\x7fn+\xc9\x1c\x8c5\xbeH,%\xb87i\xd4\x140\xba09\x8e`\xcd\xc7\x10c\xe6\x91\xa9F\x1an<'\xda\xce\xa3@\xc6\xd5\xa8w\r\xd3\x88\xe7\x85\xd31\xa4O\xd7&\x15G\xb7[C\x81\x06\xdf\x04\x90^-\xf5\xa7VF\x0eQ\xc8\x80F\xb4&Y\xd9\xa9\xda\xc1\xc2\x82\xb6Q\xc4\x88\x06d\xbd\0\xdfFS\xba\xc8\xcd3\x80uv\xe5a\x97\xa2l}n\xdc\xc4\xa4\xaf;\x7f,\"\xfa\xf4\xbc\xbb>\xafA\xe4\x80:\x8f4n\xa4\t\xf2\xf6/\x8a\xaa=\x8es\xb1\xb7\xdc\xe2\x8b/\x95l=\xc8\xd5\x80\xd8U\x0b\xe5\x07c\xd3I \xb3\xf4\x0fH\xd5G\xebQ{\x02l$X\xfe\x1a\xe1\x80\xd9_\x8e3\x08\x0b\x1e\xd3\x95q\xb3\x15`\xe1j\x10\xbe\xc9N\xbc\x85\xd3\xf1\x1a\xe1\x80\xd9^\x9as\xae'\xf9\xb4\xf2\xb2p0\xc2\x90\xab\xa8\xa0\x92\x1e\xbcE\x13\xf1H[\x80\xd9^\x19xkO\x99IC\"L\xe9u\x99\x01\xbd^\xeb\xd9\xb2\xdaf\"+\xa7\x80\xd9HZ\x17\xc5z\xd4I\x8dI0\x16\xbf!\x97U\xec#\xd8T6Z>T\xa7\x80\xd9M\x8f\xb4f\xe7\x9fhmAvO\xfcc\xf3\xca\xf3\xf0W^y\xaf\xed\xa6L\x80\xd9D\xc7\xa4\xe7\xfc\x0e\xbf\x80\n\xab\xc6!\x8d_\xa2\x1a\x17\x1c\x182\xea\xbd\x98#\x80\xd9oiT\xdd\x98\xd3\x91\xf8\x11G\xd3\x94Ut\xa2kDA\xb2\xff\xfa6\xbc\xde\x80\xd9n(\xf3>\x88e\r\xe0(\xe2\xbd\x99X\xccK\xe5\xa3n\x92F\x8930\xd6\x80\xd9bU\xb4\x9e;\x99\xad\xe4\xde\x9f\xe2\xc4D\x08;\xdaI\xdbV\xbe}s\x1a\xe1\x80\xd9\x0e\xc0E\xe6\xa6\xc4\xe9\x1f\x0e\x11V\xb7\x8a\xdfb\x13t,.\xd4\x80\xc37[\x80\xea\x95)\x87a\x85Z\x9e\x97\xb4\xa1\x05\xe4\xd7\0\xf1o\xe2/\xb2|\x90\xd3\x99\x12\xbfsy\xa8\xd0\xaf\xf6\xa3\xb8Y\xbf\x1e]\xd6!9\xed&\xc5\xf1\x98:V\xf1\xfd\xa2\xbf\x03\xe6Gc\x9a\x0e\xd9\x9f\xb3\xcb\xfd!\xf6Z\xe5E\x1a\xb8\xb7\xbc\xfd\xe1_\x9bn\xbf\x8c\xd1\xdd\xdc(\x99P\xa193\"\xa1\x07\xe59\xa2\xe7N\x89\xad\xf9,\xb9\xce\xe0\xbf\x96\xa7\x03\xc8p\xda~\xed\xb2\x96l\xa1\x08\x8a\x96A<J\x1d\xa7VF\x15\x84\xfd\xbf\x96\xa7\x03\xc8p\xda~\xed\xb2\x96l\xa1\x08v6A<J\x1d\xa7VF\x15\x84\xfd\xbe@\x9d\xe3,\xff\x0ej\xdb\x06\xe4f\xb1\x07\x95*\xe8 \x9b\x87\xad\xf9,\xa3_]\xbeA\t\xdb\x07\xa2\xdd$\xc2z\\\x94\xbcvi\xb6W\x95A\xf2\xb4\xf8\0\xa4\xd9S\xbeI\xe4\xfa\x934M\xc6\x83h\0\xd8\x01\th\x9b\xa1\xfb\x9f\x96\xa7VF\x13\x9a9\xbeg\xfd\xe6\x95\x8eB;F\xb2{\xe5\x82\x88\x8f!\xa9\xcc\xe0\xb8p\xc6\xe55.\x12\xbe\xf7c\xca\xc3\xba\xdbYr\xdc.x\x0e^\xe6\xd7\x95\x18\t\xba11\xda_\x1dY\xbdvk\xee\xf0\x85_\x19Y\xbd\x7f\x1c\x1b.\xfa \xab\xa4+-\xcf\xb4\xd2Q.\xd5\xbd\xdb\x01>\xe8y\t\xc0\xddn\xed\x06m\xd7\xa88\xf0;\xf1<\xdbO\xcc\xb1)\xae\xbd\xe9\xcb\x01\x1c`A\xc9q\xf9\xec\xbd\x1e\x06\x13^\xd0\xd7\xa0\x08\xcf\xb4\xc0\xce\x99\xc3\xbd\xefv\xfa\xd5\x03n\xba\xda]_*\x07\xa3i\xb4\xaa7xk\xb1&\x9b6\x1e\x15\xbd\xfb\xc7\xeb\x9a\xf6t[\x92K\x91\xcb\xc1\x9f\x11\x97\n:\x87'g\x8c\xb6\n\xcb\xf1\xbd\xa2#|\x96\x0e\x17p\xf6\xc8+\xaa0\x177A\xab\xddW\xa7\x9d0M\xf5\xe0!\xbd\xb27\xa9\xc0\xe8\x16\xcdP}\xc7\xa88\xa0\x0f\xfc\xfdOG\xcc\xdafj\x8f!\xd9\xbd\xb7\x10\x97\xd6\x10$\x11\xf0\x95\xean\xb1\t~ '\xde\xd6\xfc\xad\xf9,\xb7\xae\x98\xbc\x10\xf0mM\xe6^\xde\xf6\x0b\xb6\x111\t\xe9\xe7\x19\xe8u\xe7\xa7VF\x0c\xc0\x8d\xbc\x8cj\x9a\x1bE\xe5\xa8\xab*\xb3R`\x99\x8c\xc4\x03\xf0N\xbe\x05\xbd\x9dZ&\xc2\xbbI\x1a\xd2R\xda\xdf\xc03g\x89\x8aQ\x0b\xec\xcct\x88\xbb\xff\xad\xf9,\xb728\xbbN\xaf\xb1\x85\xba\xf7\x16\xe6 \xcb\xc7,\x95Vgb\xc5\xcdL\\\x7f\xa6c\x04\0\xbbQ\x85\x17\x1e\xbf\x03\x8a$\x9fo0\xf1\x06jc\xc4\xb0\xa4\xbe\xa7VF\x12\xde\\\xbbh\x08\x8cdJ\x8ci\xbaZ\xf6\0\xe0\x18zr\xf1\x90\xb7\n\xae\xaa)\x13\x1a\xe1\xbbzrbKYB=0\x85q}\xb8\xd3\xcbR\xf98\\\xeeMw\xf4\x86\x1a\xe1\xbb\x1b\xc2\xd3g\xf7%\x15\xf1\x85U\xb5\x11m\x92S\x17\x88Z]^3\xd3\xa8\x04w\xbb)1\x97X\xb5=Rf\x89\xfe\xa4\xb6i\xe8\x0cs\x98\xd5 \x05\xbd\xb7\x9b\xe9\x1f\xbb)\xef\xf7\x0bKq1\xa9\x8a\xfe\x8a\xb1\0.\xbd\xe2\x1b.\xb1\xad\xf9,\xb9`\xb6\xbb)\xef\xf7\x0bKq1\xa9\x8a\xfe\x8a\xb1\07\xbd\xe2\x1b.\xb1\xad\xf9,\xb9`\xb6\xbb.\xf3\xed\x84\xb3\x8c\xaf\xb2:\xdc\xe0$R\xd0\x1c)D\x9e\x05_\xb3zD\x1c\x0f\xbb\xc3\x19\xf3\xc4\x9f$\xdfwJ\x03\x96W\xb4]\xdd\xfa<7\x9d\x05\xbd\xb7\x9b\xf7\x0c\xbb\xc6\xa7\x0co<7\xebLV3w\xb0\x86io\x9d\x9c}oKY\xe6\xf4\x1a\xe1\xbb\xd2\x126\x07~\xa0\x86(\x9a\xb7\xe3\xef\xf3\x15\x82\x04\xd8\xe7\x0b\x05\x1f\xcdm%!\xbb\xd3\xb2\x1c\xa2\xdf\x95\x90\x93\x88\x9c\x8a3\xa4H\xca\xcd8}x3\xbf\x94\x0e\x1a\xe1\xbb\xd8\xa7N\r\x10\xa92$\x06\x0e\xfdt\xa5\xbcNCk\xa8)\x1b\x93\xce}\xe8p\xbb\xe2\xdb\\i\xd4\x92\x14\xdb\xbb\xfe{\xe5\x87\xd0\xd3$\x19S\x80g\xab\xf7\xe6\x18b\xbb\xe3\x08\xfc\xa6\xc8$\xbe\x02[\x0b\x1e\x9dw|*?\xc7m\x85\xb6\xbeeSx\xc6\xbb\xe3%\x0f{\x02\xb8\xa5\xe1\x9b\xd2\xeb\x91<\x18\xdf\xa8a\xa5\xbcYq\x94\xca\xd4s\xbb\xe3\x87\xdc\x18\x10\x03\xa3v\x18\xd5\xb1\x11\x80\x13\xda\xaczJn/\xb8r\xe7\x1a\xe1\xbb\xe0\xe8\x91\xf6\xd6\xe5XQ=\x9dx\x98\x11\xcd\xef\x0c\x84\xa8d\xb9\x15\xd8\xc6\xd15\xbb\xe0\xef3\xcdItglP\x8d\t\xcf}\xbcr\x1c\xc7o,\xb9\x15\xd8\xc6\xd0?\xbb\xe0\x93\xec\xf4\x91\xb1>\xdf\x84\xe5\xcd\xf5\x8d~\xd8\x1d\xdej\x82gjo\xf6\x1a\x95\xbb\xe6X\xe4\xa2\xf0^\x04\x7f\xfe\xd8\x9bx\x0e\x9d\xab\x05\xe5@\xaeP\xeaH;\x1fP\xbb\xe6\xcam\xc6\x87\xcd\x9d\x14\x15\nT\0\xea\x1b\xe5S\xe1Y\xb5v\xdaf\xe5\xb1\xfd\xbb\xe7\r\x90L\x9f\xc0\xe4\xa7f\x93\xf3\xd0\xc5\xc6\xc5\xb4\xf6\x06i\xac\xda(\xfa\x1a\xe1\xbb\xe4\x19\xd8x\x9fe7\x01\xee\xf8\xd1\xbcu\x85I\xd3\xd8\xc0\xb2X\x9c\x8cU?\xfa\xbb\xe4\xb2,!\xb4\xe0\x1ar\xa7\xac\xe01\xa4\xd3R\x07\x17\xf0\xec_\x8f\xca\x05\x1a\xe1\xbb\xeb\xcc\x0e\xacw\xa6\x12 \xf1*|k\xd6\xee\x8f4\xe6;\tO\x9d)0\xad\xba\xbb\xe8\xf2|\x0e\xccE\x08\x14\xd6D\x81a\x83!>\xf59\xd3\xdb\xac\xda\xbc\n\x1a\xe9\xbb\xee\xe9\x9f\x16\xc7\xc2\xda\xf7\xa9\xa2\xa0\x15\xf2\xcc\x1a`\xd1\xbaKPj\xda<\x1a\xe9\xbb\xef\x1b9\x18V~\xf6cU*x`%\xf7b+cl\xe3\x87\xb5\xd1\xd8gp\xbb\xecn\x14\xe1\xd5\t\x1b\x0f:\x8d@\xb2\x1eU>\xffS\xf6\x04TKD\x8d\xcbn\xbb\xec\xfa\xd4\xdf\xc61\xdf`\xb2\xcfK\xdd\x10\xc0\xe0\x9f*\xff\xa2\xad\xf9#(\xcc\x1b\xbb\xed\xf3p\x15\xfeB\x0e&\x9a=Vp\xd1\"\xb6X\xd3+u\x80\xc9\0h\x06h\xbb\xed\xbd\r\xf5\\5(\xfey\xcf\xec\x18\xac\xae\xbe4\xa0&\xdc\xaeJXH\xc4\x91\xbb\xf2\x1c\x17\xfc\xe4\x86;\xbb?\xa1?5g\x03\xb3\xf4\x91@\x7f\xbd\xa3\x84\x86\xc4\x91\xbb\xf2\x925G:\x83\xdbp\xc5QO\xbaz\xe77n\x02y\xcc\xb2\xa4\xdb\xd4\xce%\xbb\xf2";

        let _rst = decode(data);
        println!("{:#?}", _rst);
    }
}
