use bytes::Bytes;
use yiilian_core::data::BencodeData;

#[derive(Debug, Clone)]
pub struct KrpcFrame {
    kind: FrameKind,
    data: Bytes,
}

#[derive(Debug, Clone)]
pub enum FrameKind {
    Empty,
    Query(Query),
    Reply(Reply),
    RError(BencodeData),
}

#[derive(Debug, Clone)]
pub enum Query {
    Ping(BencodeData),
    FindNode(BencodeData),
    GetPeers(BencodeData),
    AnnouncePeer(BencodeData),
}

#[derive(Debug, Clone)]
pub enum Reply {
    PingOrAnnounce(BencodeData),
    FindNode(BencodeData),
    GetPeers(BencodeData),
}