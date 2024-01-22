
#[derive(Debug, Clone)]
pub enum RequestBody {
    Query(Query),
    Reply(Reply),
    RError(RError)
}

#[derive(Debug, Clone)]
pub enum ResponseBody {
    Query(Query),
    Reply(Reply),
    RError(RError)
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