use std::net::SocketAddr;

#[derive(Debug)]
pub struct Response<T> {
    pub body: T,
    pub remote_addr: SocketAddr,
    pub local_addr: SocketAddr,
}

impl<T> Response<T> {
    pub fn new(body: T, remote_addr: SocketAddr, local_addr: SocketAddr) -> Self {
        Self {
            body,
            remote_addr,
            local_addr,
        }
    }
}