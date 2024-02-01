use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Request<T> {
    pub body: T,
    pub remote_addr: SocketAddr,
    pub local_addr: SocketAddr,
}

impl<T> Request<T> {
    pub fn new(body: T, remote_addr: SocketAddr, local_addr: SocketAddr) -> Self {
        Self {
            body,
            remote_addr,
            local_addr,
        }
    }
}

unsafe impl<T> Send for Request<T> {}