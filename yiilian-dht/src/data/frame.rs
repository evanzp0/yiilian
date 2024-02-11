use std::{collections::BTreeMap, fmt::Display};

use bytes::Bytes;
use yiilian_core::{common::error::Error, data::BencodeData};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Frame(pub BTreeMap<Bytes, BencodeData>);

impl Frame {
    pub fn new() -> Self {
        Frame(BTreeMap::new())
    }

    pub fn get(&self, key: &str) -> Option<&BencodeData> {
        self.0.get(key.as_bytes().into())
    }

    /// 检查 frame 中是否有存在和 items 中相同的 key + value 条目。
    /// items 是一个 (key, value) 列表
    pub fn is_exist_items(&self, items: &[(&str, &str)]) -> bool {
        for (key, val) in items {
            if let Some(v) = self.0.get(key.as_bytes()) {
                match v.as_bstr() {
                    Ok(v) => {
                        if v != *val {
                            return false;
                        }
                    }
                    Err(_) => return false,
                }
            } else {
                return false;
            }
        }

        true
    }
}

impl Display for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TryFrom<BencodeData> for Frame {
    type Error = Error;

    fn try_from(value: BencodeData) -> Result<Self, Self::Error> {
        match value {
            BencodeData::Map(m) => Ok(Frame(m)),
            _ => {
                Err(Error::new_frame(None, Some(format!("Data is invalid to convert to frame: {value}"))))?
            },
        }
    }
}

impl From<Frame> for BencodeData {
    fn from(value: Frame) -> Self {
        BencodeData::Map(value.0)
    }
}
