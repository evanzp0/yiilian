use std::{net::SocketAddr, sync::Arc};

use tokio::net::UdpSocket;
use tower::Service;

use crate::{
    common::error::Error,
    data::{Request, Response}, udp::net::send_to,
};

use super::net::recv_from;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Server<S> {
    pub socket: Arc<UdpSocket>,
    pub local_addr: SocketAddr,
    pub recv_filter: S,
    ctx_index: i32,
}

impl<S> Server<S>
where
    S: Service<Request, Response = Response> + Clone,
    S::Error: Send + std::fmt::Debug + 'static,
    S::Future: Send + 'static,
{
    pub fn new(ctx_index: i32, socket: Arc<UdpSocket>, recv_filter: S) -> Self {
        let local_addr = socket.local_addr().expect("Get local address error");
        Server {
            ctx_index,
            socket,
            recv_filter,
            local_addr,
        }
    }

    pub async fn run_loop(&mut self) -> Result<()> {
        loop {
            let recv_rst = recv_from(&self.socket).await;

            match recv_rst {
                Ok((data, remote_addr)) => {
                    let req = Request::new(data, remote_addr, self.local_addr);
                    let fut = self.recv_filter.call(req);
                    let socket = self.socket.clone();
                    let ctx_index = self.ctx_index;

                    tokio::spawn(async move {
                        let rst = fut.await;
                        match rst {
                            Ok(res) => {
                                send_to(socket, &res.data, res.remote_addr).await.ok();
                            }
                            Err(e) => {
                                log::error!(target:"yiilian_core::udp::server::run_loop", "Send Error:  [index: {}] {:?}", ctx_index, e)
                            },
                        }
                    });
                }
                Err(e) => {
                    log::error!(target:"yiilian_core::udp::server::run_loop", "Recv failed: [index: {}] {:?}", self.ctx_index, e)
                }
            }
        }
    }
}

