use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct BtTorrent {
    announce: String,
    info: MetaInfo,
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
    length: usize,
    path: String,
}