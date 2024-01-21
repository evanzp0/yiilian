use std::fmt::Debug;
use std::net::ToSocketAddrs;
use std::{net::SocketAddr, sync::Arc};

use bytes::Bytes;
use futures::FutureExt;
use tokio::net::UdpSocket;
use yiilian_core::common::error::{Error, trace_panic};
use yiilian_core::data::{Body, Request};
use yiilian_core::net::io::send_to;

use crate::data::raw_body::RawBody;

use super::service::raw_service::RawService;


type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Server<S> {
    pub socket: Arc<UdpSocket>,
    pub local_addr: SocketAddr,
    pub recv_service: S,
    pub ctx_index: i32,
}

impl<S> Server<S>
where
    S: RawService<RawBody, ResBody = RawBody> + Clone + Send + 'static,
    S::Error: Debug + Send,
{
    pub fn new(ctx_index: i32, socket: Arc<UdpSocket>, recv_filter: S) -> Self {
        let local_addr = socket.local_addr().expect("Get local address error");
        Server {
            ctx_index,
            socket,
            recv_service: recv_filter,
            local_addr,
        }
    }

    /// 通过绑定方式，生成 UdpIo
    pub fn bind<A: ToSocketAddrs>(ctx_index: i32, socket_addr: A, recv_filter: S) -> Result<Self> {
        let std_sock = std::net::UdpSocket::bind(socket_addr)
            .map_err(|e| Error::new_bind(Some(Box::new(e))))?;
        std_sock
            .set_nonblocking(true)
            .map_err(|e| Error::new_bind(Some(Box::new(e))))?;

        let socket =
            UdpSocket::from_std(std_sock).map_err(|e| Error::new_bind(Some(Box::new(e))))?;
        let socket = Arc::new(socket);

        Ok(Server::new(ctx_index, socket, recv_filter))
    }

    pub async fn run_loop(&self) {
        loop {
            let mut buf = [0; 65000];
            let rst = self
                .socket
                .recv_from(&mut buf)
                .await
                .map_err(|e| Error::new_io(Some(e.into()), self.socket.local_addr().ok()));

            let (len, remote_addr) = match rst {
                Ok(rst) => rst,
                Err(error) => {
                    log::debug!(
                        target: "yiilian_dht::net::server", 
                        "recv error: [index: {}] {:?}", 
                        self.ctx_index, error
                    );
                    continue;
                }
            };
            let local_addr = self.local_addr;
            let data: Bytes = buf[..len].to_owned().into();
            let req = Request::new(RawBody::new(data), remote_addr, local_addr);

            let service = self.recv_service.clone();
            let socket = self.socket.clone();
            let ctx_index = self.ctx_index;

            // 每个收到的连接都会在独立的任务中处理
            tokio::spawn(async move {
                let rst = service.call(req).catch_unwind().await;
                match rst {
                    Ok(rst) => match rst {
                        Ok(mut res) => {
                            match send_to(socket, &res.data(), res.remote_addr).await {
                                Ok(_) => {
                                    log::trace!(
                                        target: "yiilian_dht::net", 
                                        "send {} bytes to address: [index: {}] {}",
                                        ctx_index, res.len(), res.remote_addr
                                    );
                                },
                                Err(error) => {
                                    log::debug!(
                                        target: "yiilian_dht::net", 
                                        "send_to error: [index: {}] {:?}", 
                                        ctx_index, error
                                    );
                                },
                            }
                        }
                        Err(error) => {
                            log::debug!(
                                target: "yiilian_dht::net::server", 
                                "service error: [index: {}] {:?}", 
                                ctx_index, error
                            );
                        }
                    },
                    Err(error) => {
                        // 捕获 panic 后的处理
                        let (b, err) = trace_panic(&error);
                        log::debug!(
                            target: "yiilian_dht::net::server",
                            "service panic: [index: {}] {}\ntrace:\n{:?}",
                            ctx_index, err, b
                        );
                    }
                }
            });
        }
    }
}
