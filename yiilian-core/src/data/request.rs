use std::net::SocketAddr;

#[derive(Debug)]
pub struct Request<B> {
    pub data: B,
    pub remote_addr: SocketAddr,
    pub dir: IoDir,
}

#[derive(Debug, PartialEq, Eq)]
pub enum IoDir {
    Send,
    Recv,
}
