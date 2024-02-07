
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Semaphore;
use yiilian_core::common::error::Error;
use yiilian_core::common::error::Kind;
use yiilian_core::common::expect_log::ExpectLog;
use yiilian_core::data::{Body, Request};
use yiilian_core::net::udp::recv_from;
use yiilian_core::net::udp::send_to;

use crate::data::body::{BodyKind, KrpcBody};

use crate::service::KrpcService;

pub struct Server<S> {
    socket: Arc<UdpSocket>,
    local_addr: SocketAddr,
    recv_service: S,
    /// 最大并发任务数
    workers: Option<Arc<Semaphore>>,
}

impl<S> Server<S>
where
    // S: MakeServiceRef<Context, KrpcBody, ResBody = KrpcBody>,
    // S::Service: Send + 'static,
    // S::Error: Debug + Send,
    S: KrpcService<KrpcBody, ResBody = KrpcBody, Error = Error> + Clone + Send + 'static,
{
    pub fn new(socket: Arc<UdpSocket>, recv_service: S, workers: Option<Arc<Semaphore>>) -> Self {
        let local_addr = socket.local_addr().expect("Get local address error");
        Server {
            socket,
            recv_service,
            local_addr,
            workers,
        }
    }

    pub async fn run_loop(&self) -> Result<(), Error> {
        let local_port = self.local_addr.port();
        let local_addr = self.local_addr;

        loop {
            // worker 用信号量来限制并发的最大任务数
            let worker = match self.workers.clone() {
                Some(val) => {
                    let val = val.acquire_owned()
                        .await
                        .expect_error("Semaphore acquire_owned failed");
                    
                    Some(val)
                },
                None => {
                    None
                },
            };

            let (data, remote_addr) = match recv_from(&self.socket).await {
                Ok(val) => val,
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

            if remote_addr.port() == 0 {
                log::trace!(
                    target: "yiilian_dht::net::server",
                    "recv_from remote address is invalid: [{}] {}",
                    local_port, remote_addr
                );
                continue;
            }

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
                    }
                };
                Request::new(body, remote_addr, local_addr)
            };

            let socket = self.socket.clone();
            let mut service = self.recv_service.clone();

            // 每个收到的连接都会在独立的任务中处理
            tokio::spawn(async move {
                let rst = service.call(req).await;
                match rst {
                    Ok(mut res) => {
                        match res.body.get_kind() {
                            BodyKind::Empty => {} // response body 为空则不需要 send_to
                            _ => {
                                if let Err(error) =
                                    send_to(&socket, &res.get_data(), res.remote_addr).await
                                {
                                    log::error!(
                                        target: "yiilian_dht::net::server",
                                        "send_to error: [{}] {:?}\n req:\n",
                                        local_port,
                                        error
                                    );

                                    // log::error!(
                                    //     target: "yiilian_dht::net::server",
                                    //     "send_to error: [{}] {:?}\n req:\n{:?}",
                                    //     local_port,
                                    //     error,
                                    //     req
                                    // );
                                }
                            }
                        }
                    }
                    Err(error) => match error.get_kind() {
                        Kind::Block => {}
                        Kind::Token => {}
                        Kind::General => {}
                        _ => {
                            log::debug!(
                                target: "yiilian_dht::net::server",
                                "service error: [{}] {:?}",
                                local_port, error
                            );
                        }
                    },
                }

                drop(worker)
            });
        }
    }
}
