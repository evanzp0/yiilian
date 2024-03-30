use bytes::Bytes;
use yiilian_core::{common::error::Error, data::BencodeData};

#[derive(Debug, Clone)]
pub struct BtTorrent {
    pub announce: String,
    pub info: MetaInfo,
}

#[derive(Debug, Clone)]
pub enum MetaInfo {
    SingleFile {
        length: usize,
        name: String,
        pieces: Bytes,
        piece_length: usize,
    },
    MultiFile {
        files: Vec<MultiFile>,
        name: String,
        pieces: Bytes,
        piece_length: usize,
    },
}

#[derive(Debug, Clone)]
pub struct MultiFile {
    pub length: usize,
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

        let info = if let Some(info) = data.get(&b"info"[..]) {
            if info.has_key("length") {
                let info = info.as_map()?;
                let length = info
                    .get(&b"length"[..])
                    .unwrap_or(&BencodeData::Int(0))
                    .as_int()?
                    as usize;
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

                MetaInfo::SingleFile { length, name, pieces, piece_length }

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
                            .as_int()?
                            as usize;
                        let path = if let Some(path) = item.get(&b"path"[..]) {
                            let path = path.as_bstr()?;
                            unsafe { String::from_utf8_unchecked(path.to_vec()) }
                        } else {
                            Err(Error::new_decode(&format!(
                                "BtTorrent 'name' field decode error : {:?}",
                                value
                            )))?
                        };

                        let tmp_file = MultiFile {
                            length,
                            path,
                        };
                        tmp_files.push(tmp_file);
                    }

                    tmp_files
                } else {
                    Err(Error::new_decode(&format!(
                        "BtTorrent 'files' field decode error : {:?}",
                        value
                    )))?
                };

                MetaInfo::MultiFile { files, name, pieces, piece_length }
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

        Ok( BtTorrent { announce, info } )
    }
}
