
use std::{net::SocketAddr, sync::Arc};
use bytes::Bytes;
use futures::FutureExt;
use tokio::net::UdpSocket;
use yiilian_core::common::error::trace_panic;
use yiilian_core::common::error::Error;
use yiilian_core::common::error::Kind;
use yiilian_core::data::{Body, Request};
use yiilian_core::net::udp::send_to;

use crate::data::body::{BodyKind, KrpcBody};

use crate::service::KrpcService;

pub struct Server<S> {
    socket: Arc<UdpSocket>,
    local_addr: SocketAddr,
    recv_service: S,
}

impl<S> Server<S>
where
    // S: MakeServiceRef<Context, KrpcBody, ResBody = KrpcBody>,
    // S::Service: Send + 'static,
    // S::Error: Debug + Send,
    S: KrpcService<KrpcBody, ResBody = KrpcBody, Error = Error> + Clone + Send + 'static,
{
    pub fn new(
        socket: Arc<UdpSocket>,
        recv_service: S,
    ) -> Self {

        let local_addr = socket.local_addr().expect("Get local address error");
        Server {
            socket,
            recv_service,
            local_addr,
        }
    }

    pub async fn run_loop(&self) -> Result<(), Error> {
        let local_port = self.local_addr.port();
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
                        "recv error: [{}] {:?}",
                        local_port, 
                        error
                    );
                    continue;
                }
            };
            let local_addr = self.local_addr;
            let data: Bytes = buf[..len].to_owned().into();
            let req = {
                let body = match KrpcBody::from_bytes(data) {
                    Ok(val) => val,
                    Err(error) => {
                        log::trace!(
                            target: "yiilian_dht::net::server",
                            "Parse krpc body error: [{}] {:?}",
                            local_port, error
                        );
                        continue;
                    },
                };
                Request::new(body, remote_addr, local_addr)
            };

            let socket = self.socket.clone();
            let service = self.recv_service.clone();

            // 每个收到的连接都会在独立的任务中处理
            tokio::spawn(async move {
                let rst = service.call(req).catch_unwind().await;
                match rst {
                    Ok(rst) => match rst {
                        Ok(mut res) => {
                            match res.body.get_kind() {
                                BodyKind::Empty => {}, // response body 为空则不需要 send_to
                                _=> {
                                    if let Err(error) = send_to(&socket, &res.get_data(), res.remote_addr).await
                                    {
                                        match error.get_kind() {
                                            Kind::Conntrack => {
                                                log::error!(
                                                    target: "yiilian_dht::net::server",
                                                    "send_to error: [{}] {:?}",
                                                    local_port,
                                                    error
                                                );
                                            },
                                            _ => {
                                                log::debug!(
                                                    target: "yiilian_dht::net::server",
                                                    "send_to error: [{}] {:?}",
                                                    local_port,
                                                    error
                                                );
                                            },
                                        }
                                    }
                                },
                            }
                        }
                        Err(error) => {
                            match error.get_kind() {
                                Kind::Block => {},
                                Kind::Token => {},
                                _ => {
                                    log::debug!(
                                        target: "yiilian_dht::net::server",
                                        "service error: [{}] {:?}",
                                        local_port, error
                                    );
                                }
                            }
                        }
                    },
                    Err(error) => {
                        // 捕获 panic 后的处理
                        let (b, err) = trace_panic(&error);
                        log::error!(
                            target: "yiilian_dht::net::server",
                            "service panic: [{}] {}\ntrace:\n{:?}",
                            local_port, err, b
                        );
                    }
                }
            });
        }
    }
}
