use std::fmt;

use bytes::Bytes;
use hex::ToHex;
use sha1::{Digest, Sha1};
use crate::{
    common::error::Error,
    data::{BencodeData, Encode},
};

#[derive(Debug, Clone)]
pub struct BtTorrent {
    pub info_hash: String,
    pub announce: String,
    pub info: MetaInfo,
}

#[derive(Clone)]
pub enum MetaInfo {
    SingleFile {
        length: i64,
        name: String,
        pieces: Bytes,
        piece_length: usize,
    },
    MultiFile {
        files: Vec<FileInfo>,
        name: String,
        pieces: Bytes,
        piece_length: usize,
    },
}

impl fmt::Debug for MetaInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_map();

        match self {
            MetaInfo::SingleFile {
                length,
                name,
                pieces,
                piece_length,
            } => {
                f.entry(&"length", length);
                f.entry(&"name", name);
                f.entry(&"pieces", &format!("...({} bytes)...", pieces.len()));
                f.entry(&"piece length", piece_length);
            }
            MetaInfo::MultiFile {
                files,
                name,
                pieces,
                piece_length,
            } => {
                f.entry(&"files", files);
                f.entry(&"name", name);
                f.entry(&"pieces", &format!("...({} bytes)...", pieces.len()));
                f.entry(&"piece length", piece_length);
            }
        }

        f.finish()
    }
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub length: i64,
    pub path: String,
}

impl TryFrom<&[u8]> for BtTorrent {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let data = BencodeData::parse(value)?;

        let data = data.as_map()?;
        let announce = if let Some(announce) = data.get(&b"announce"[..]) {
            let tmp = announce.as_bstr()?;
            unsafe { String::from_utf8_unchecked(tmp.to_vec()) }
        } else {
            "".to_owned()
        };

        let (info, info_hash) = if let Some(info) = data.get(&b"info"[..]) {
            let info_hash: String = {
                // info.encode().encode_hex_upper()
                let metadata_info = info.encode();
                let mut hasher = Sha1::new();
                hasher.update(&metadata_info);
                let i_hash = hasher.finalize().to_vec();
                
                i_hash.encode_hex_upper()
            };

            if info.has_key("length") {
                let info = info.as_map()?;
                let length = info
                    .get(&b"length"[..])
                    .unwrap_or(&BencodeData::Int(0))
                    .as_int()? as usize;
                let name = if let Some(name) = info.get(&b"name"[..]) {
                    let name = name.as_bstr()?;
                    unsafe { String::from_utf8_unchecked(name.to_vec()) }
                } else {
                    Err(Error::new_decode(&format!(
                        "BtTorrent 'name' field decode error : {:?}",
                        value
                    )))?
                };
                let piece_length = if let Some(piece_length) = info.get(&b"piece length"[..]) {
                    piece_length.as_int()? as usize
                } else {
                    Err(Error::new_decode(&format!(
                        "BtTorrent 'piece length' field decode error : {:?}",
                        value
                    )))?
                };
                let pieces = if let Some(pieces) = info.get(&b"pieces"[..]) {
                    pieces.as_bstr()?.to_owned()
                } else {
                    Err(Error::new_decode(&format!(
                        "BtTorrent 'pieces' field decode error : {:?}",
                        value
                    )))?
                };

                let length = length as i64;
                (
                    MetaInfo::SingleFile {
                        length,
                        name,
                        pieces,
                        piece_length,
                    },
                    info_hash,
                )
            } else if info.has_key("files") {
                let info = info.as_map()?;
                let name = if let Some(name) = info.get(&b"name"[..]) {
                    let name = name.as_bstr()?;
                    unsafe { String::from_utf8_unchecked(name.to_vec()) }
                } else {
                    Err(Error::new_decode(&format!(
                        "BtTorrent 'name' field decode error : {:?}",
                        value
                    )))?
                };
                let piece_length = if let Some(piece_length) = info.get(&b"piece length"[..]) {
                    piece_length.as_int()? as usize
                } else {
                    Err(Error::new_decode(&format!(
                        "BtTorrent 'piece length' field decode error : {:?}",
                        value
                    )))?
                };
                let pieces = if let Some(pieces) = info.get(&b"pieces"[..]) {
                    pieces.as_bstr()?.to_owned()
                } else {
                    Err(Error::new_decode(&format!(
                        "BtTorrent 'pieces' field decode error : {:?}",
                        value
                    )))?
                };
                let files = if let Some(files) = info.get(&b"files"[..]) {
                    let files = files.as_list()?;
                    let mut tmp_files = vec![];

                    for item in files {
                        let item = item.as_map()?;

                        let length = item
                            .get(&b"length"[..])
                            .unwrap_or(&BencodeData::Int(0))
                            .as_int()? as usize;
                        let path = if let Some(path) = item.get(&b"path"[..]) {
                            let path = path.as_list()?;
                            let path = path[0].as_bstr()?;
                            unsafe { String::from_utf8_unchecked(path.to_vec()) }
                        } else {
                            Err(Error::new_decode(&format!(
                                "BtTorrent 'name' field decode error : {:?}",
                                value
                            )))?
                        };
                        let length = length as i64;

                        let tmp_file = FileInfo { length, path };
                        tmp_files.push(tmp_file);
                    }

                    tmp_files
                } else {
                    Err(Error::new_decode(&format!(
                        "BtTorrent 'files' field decode error : {:?}",
                        value
                    )))?
                };

                (
                    MetaInfo::MultiFile {
                        files,
                        name,
                        pieces,
                        piece_length,
                    },
                    info_hash,
                )
            } else {
                Err(Error::new_decode(&format!(
                    "BtTorrent 'info' field decode error : {:?}",
                    value
                )))?
            }
        } else {
            Err(Error::new_decode(&format!(
                "BtTorrent 'info' field not found : {:?}",
                value
            )))?
        };

        Ok(BtTorrent { announce, info, info_hash })
    }
}
