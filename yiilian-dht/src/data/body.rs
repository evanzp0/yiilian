use std::net::SocketAddr;

use bytes::Bytes;
use yiilian_core::data::{decode, Body, Encode};
use yiilian_core::{common::error::Error, data::BencodeFrame as Frame};

use crate::common::Id;
use crate::transaction::TransactionId;

use super::{
    announce_peer::AnnouncePeer, error::RError, find_node::FindNode,
    find_node_reply::FindNodeReply, get_peers::GetPeers, get_peers_reply::GetPeersReply,
    ping::Ping, ping_announce_replay::PingOrAnnounceReply,
};

#[derive(Debug, Clone)]
pub struct KrpcBody {
    kind: BodyKind,
    data: Bytes,
}

impl KrpcBody {
    pub fn new(kind: BodyKind) -> Self {
        let data = {
            let data: Frame = match kind.clone() {
                BodyKind::Empty => Frame::Empty,
                BodyKind::Query(val) => val.into(),
                BodyKind::Reply(val) => val.into(),
                BodyKind::RError(val) => val.into(),
            };

            data.encode()
        };

        Self { kind, data }
    }

    pub fn from_bytes(data: Bytes) -> Result<Self, Error> {
        let frame: Frame = decode(&*data)?;

        let kind: BodyKind = frame.try_into()?;

        Ok(Self { kind, data })
    }

    pub fn get_kind(&self) -> &BodyKind {
        &self.kind
    }
}

#[derive(Debug, Clone)]
pub enum BodyKind {
    Empty,
    Query(Query),
    Reply(Reply),
    RError(RError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
    Ping(Ping),
    FindNode(FindNode),
    GetPeers(GetPeers),
    AnnouncePeer(AnnouncePeer),
}

#[derive(Debug, Clone)]
pub enum Reply {
    PingOrAnnounce(PingOrAnnounceReply),
    FindNode(FindNodeReply),
    GetPeers(GetPeersReply),
}

impl Default for BodyKind {
    fn default() -> Self {
        BodyKind::Empty
    }
}

impl Default for KrpcBody {
    fn default() -> Self {
        Self {
            kind: BodyKind::default(),
            data: Default::default(),
        }
    }
}

impl Body for KrpcBody {
    type Data = Bytes;

    fn get_data(&mut self) -> Self::Data {
        let s = std::mem::take(&mut *self);
        s.data
    }

    fn len(&self) -> usize {
        self.data.len()
    }
}

impl Query {
    pub fn get_tid(&self) -> TransactionId {
        match self {
            Query::Ping(val) => val.t.clone(),
            Query::FindNode(val) => val.t.clone(),
            Query::GetPeers(val) => val.t.clone(),
            Query::AnnouncePeer(val) => val.t.clone(),
        }
    }

    pub fn get_sender_id(&self) -> Id {
        match self {
            Query::Ping(val) => val.id.to_owned(),
            Query::FindNode(val) => val.id.to_owned(),
            Query::GetPeers(val) => val.id.to_owned(),
            Query::AnnouncePeer(val) => val.id.to_owned(),
        }
    }

    pub fn is_read_only(&self) -> bool {
        let ro = match self {
            Query::Ping(val) => val.ro,
            Query::FindNode(val) => val.ro,
            Query::GetPeers(val) => val.ro,
            Query::AnnouncePeer(val) => val.ro,
        };

        match ro {
            Some(val) => {
                if val == 1 {
                    true
                } else {
                    false
                }
            }
            None => false,
        }
    }
}

impl Reply {
    pub fn get_tid(&self) -> TransactionId {
        match self {
            Reply::PingOrAnnounce(val) => val.t.clone(),
            Reply::FindNode(val) => val.t.clone(),
            Reply::GetPeers(val) => val.t.clone(),
        }
    }

    pub fn get_id(&self) -> Id {
        match self {
            Reply::PingOrAnnounce(val) => val.id.clone(),
            Reply::FindNode(val) => val.id.clone(),
            Reply::GetPeers(val) => val.id.clone(),
        }
    }

    pub fn get_ip(&self) -> Option<SocketAddr> {
        match self {
            Reply::PingOrAnnounce(val) => val.ip.clone(),
            Reply::FindNode(val) => val.ip.clone(),
            Reply::GetPeers(val) => val.ip.clone(),
        }
    }

    pub fn to_frame(self) -> Frame {
        match self {
            Reply::PingOrAnnounce(val) => val.into(),
            Reply::FindNode(val) => val.into(),
            Reply::GetPeers(val) => val.into(),
        }
    }
}

impl TryFrom<Frame> for BodyKind {
    type Error = Error;

    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        if let Frame::Map(ref m) = frame {
            if frame.verify_items(&[("y", "q")]) {
                if frame.verify_items(&[("q", "ping")]) {
                    return Ok(BodyKind::Query(Query::Ping(frame.try_into()?)));
                } else if frame.verify_items(&[("q", "find_node")]) {
                    return Ok(BodyKind::Query(Query::FindNode(frame.try_into()?)));
                } else if frame.verify_items(&[("q", "get_peers")]) {
                    return Ok(BodyKind::Query(Query::GetPeers(frame.try_into()?)));
                } else if frame.verify_items(&[("q", "announce_peer")]) {
                    return Ok(BodyKind::Query(Query::AnnouncePeer(frame.try_into()?)));
                }
            } else if frame.verify_items(&[("y", "r")]) {
                if let Some(params) = m.get(&b"r"[..]) {
                    if params.has_key("token") {
                        return Ok(BodyKind::Reply(Reply::GetPeers(frame.try_into()?)));
                    } else if params.has_key("nodes") {
                        return Ok(BodyKind::Reply(Reply::FindNode(frame.try_into()?)));
                    } else {
                        return Ok(BodyKind::Reply(Reply::PingOrAnnounce(frame.try_into()?)));
                    }
                }
            } else if frame.verify_items(&[("y", "e")]) {
                return Ok(BodyKind::RError(frame.try_into()?));
            }
        }

        Err(Error::new_frame(
            None,
            Some(format!("convert Frame to KrpcBody failed: {frame}")),
        ))
    }
}

impl TryFrom<Frame> for KrpcBody {
    type Error = Error;

    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        let kind: BodyKind = frame.try_into()?;
        Ok(KrpcBody::new(kind))
    }
}

impl From<Query> for Frame {
    fn from(value: Query) -> Self {
        match value {
            Query::Ping(val) => val.into(),
            Query::FindNode(val) => val.into(),
            Query::GetPeers(val) => val.into(),
            Query::AnnouncePeer(val) => val.into(),
        }
    }
}

impl From<Reply> for Frame {
    fn from(value: Reply) -> Self {
        match value {
            Reply::PingOrAnnounce(val) => val.into(),
            Reply::FindNode(val) => val.into(),
            Reply::GetPeers(val) => val.into(),
        }
    }
}

impl From<BodyKind> for Frame {
    fn from(value: BodyKind) -> Self {
        match value {
            BodyKind::Empty => Frame::Empty,
            BodyKind::Query(val) => val.into(),
            BodyKind::Reply(val) => val.into(),
            BodyKind::RError(val) => val.into(),
        }
    }
}

impl From<KrpcBody> for Frame {
    fn from(value: KrpcBody) -> Self {
        match value.kind {
            BodyKind::Empty => Frame::Empty,
            BodyKind::Query(val) => val.into(),
            BodyKind::Reply(val) => val.into(),
            BodyKind::RError(val) => val.into(),
        }
    }
}

impl From<Query> for KrpcBody {
    fn from(value: Query) -> Self {
        KrpcBody::new(BodyKind::Query(value))
    }
}

impl From<Reply> for KrpcBody {
    fn from(value: Reply) -> Self {
        KrpcBody::new(BodyKind::Reply(value))
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use yiilian_core::data::{BencodeFrame as Frame, Encode};

    use crate::data::announce_peer::AnnouncePeer;

    use super::KrpcBody;

    #[test]
    fn test_bytes_to_body() {
        let af = AnnouncePeer::new(
            "id000000000000000001".try_into().unwrap(),
            "info0000000000000001".try_into().unwrap(),
            Some(1),
            80,
            "01".into(),
            "t1".into(),
            Some("v1".into()),
            Some("127.0.0.1:80".parse().unwrap()),
            Some(1),
        );

        let frame: Frame = af.clone().into();
        let data: Bytes = frame.encode();
        let body = KrpcBody::from_bytes(data).unwrap();
        let rst: Frame = body.into();

        assert_eq!(frame, rst)
    }
}
