use std::sync::Arc;

use tokio::net::UdpSocket;
use yiilian_core::{common::error::Error, data::{Body, Request}, net::udp::send_to};

use crate::data::body::KrpcBody;

pub struct Client {
    socket: Arc<UdpSocket>,
}

impl Client {
    pub fn new(socket: Arc<UdpSocket>) -> Self {
        Client {
            socket
        }
    }

    pub async fn send(&self, mut req: Request<KrpcBody>) -> Result<usize, Error> {
        let dest = req.remote_addr;
        let data = req.get_data();
        
        send_to(&self.socket, &data, dest).await
    }
}