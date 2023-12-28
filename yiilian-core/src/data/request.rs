use std::net::SocketAddr;

#[derive(Debug)]
pub struct Request<Body> {
    pub data: Body,
    pub remote_addr: SocketAddr,
    pub dir: IoDir,
}

#[derive(Debug, PartialEq, Eq)]
pub enum IoDir {
    Send,
    Recv,
}
