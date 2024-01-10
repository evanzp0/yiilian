use std::net::SocketAddr;

use bytes::Bytes;

#[derive(Debug)]
pub struct Request {
    pub data: Bytes,
    pub remote_addr: SocketAddr,
    pub local_addr: SocketAddr,
}

#[derive(Debug, PartialEq, Eq)]
pub enum IoDir {
    Send,
    Recv,
}

impl Request {
    pub fn new(data: Bytes, remote_addr: SocketAddr, local_addr: SocketAddr) -> Self {
        Self {
            data,
            remote_addr,
            local_addr,
        }
    }
}