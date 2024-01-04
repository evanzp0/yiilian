use std::net::SocketAddr;

#[derive(Debug)]
pub struct Request<B> {
    pub data: B,
    pub remote_addr: SocketAddr,
    pub local_addr: SocketAddr,
    pub dir: IoDir,
}

#[derive(Debug, PartialEq, Eq)]
pub enum IoDir {
    Send,
    Recv,
}

impl<B> Request<B> {
    pub fn new(data: B, remote_addr: SocketAddr, local_addr: SocketAddr, dir: IoDir) -> Self {
        Self {
            data,
            remote_addr,
            local_addr,
            dir,
        }
    }
}