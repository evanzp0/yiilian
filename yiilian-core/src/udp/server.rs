use std::net::ToSocketAddrs;
use std::task::Poll;
use std::{net::SocketAddr, sync::Arc};

use bytes::Bytes;
use futures::Future;
use pin_project::pin_project;
use tokio::{io::ReadBuf, net::UdpSocket};
use tower::Service;

use crate::data::Body;
use crate::ready;

use crate::{
    common::error::Error,
    data::{Request, Response, UdpBody},
};

use super::net::send_to;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
#[pin_project]
pub struct Server<S> {
    pub socket: Arc<UdpSocket>,
    pub local_addr: SocketAddr,
    pub recv_filter: S,
    pub ctx_index: i32,

    // 重试次数
    retry_times: u8,
}

impl<S> Server<S> {
    pub fn new(ctx_index: i32, socket: Arc<UdpSocket>, recv_filter: S) -> Self {
        let local_addr = socket.local_addr().expect("Get local address error");
        Server {
            ctx_index,
            socket,
            recv_filter,
            local_addr,
            retry_times: 0,
        }
    }

    /// 通过绑定方式，生成 UdpIo
    pub fn bind<A: ToSocketAddrs>(
        ctx_index: i32,
        socket_addr: A,
        recv_filter: S,
    ) -> Result<Self> {
        let std_sock =
            std::net::UdpSocket::bind(socket_addr).map_err(|e| Error::new_bind(Some(Box::new(e))))?;
        std_sock
            .set_nonblocking(true)
            .map_err(|e| Error::new_bind(Some(Box::new(e))))?;

        let socket = UdpSocket::from_std(std_sock).map_err(|e| Error::new_bind(Some(Box::new(e))))?;
        let socket = Arc::new(socket);

        Ok(Server::new(ctx_index, socket, recv_filter))
    }
}

impl<S> Future for Server<S>
where
    S: Service<Request<UdpBody>, Response = Response<UdpBody>> + Clone,
    S::Error: Send + std::fmt::Debug + 'static,
    S::Future: Send + 'static,
{
    type Output = Result<()>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let me = self.project();
        loop {
            let mut buf = [0; 65000];
            let mut buf = ReadBuf::new(&mut buf);

            let rst = ready!(me.socket.poll_recv_from(cx, &mut buf))
                .map_err(|e| Error::new_io(Some(e.into()), me.socket.peer_addr().ok()));

            let remote_addr = match rst {
                Ok(remote_addr) => {
                    if *me.retry_times > 0 {
                        *me.retry_times = 0;
                    }
                    remote_addr
                }
                Err(error) => {
                    if *me.retry_times >= 3 {
                        log::error!(target: "yiilian_dht::udp::server", "recv_from error: [index: {}] {:?}", *me.ctx_index, error);
                        return Poll::Ready(Err(error));
                    } else {
                        *me.retry_times += 1;
                        continue;
                    }
                }
            };

            let local_addr = *me.local_addr;
            let data: Bytes = buf.filled().to_owned().into();
            let req = Request::new(UdpBody::new(data), remote_addr, local_addr);

            match me.recv_filter.poll_ready(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(_) => {
                    let fut = me.recv_filter.call(req);
                    let socket = me.socket.clone();
                    let ctx_index = *me.ctx_index;
                    // 每个收到的连接都会在独立的任务中处理
                    tokio::spawn(async move {
                        match fut.await {
                            Ok(mut res) => {
                                if let Err(error) = send_to(socket, &res.data(), res.remote_addr).await {
                                    log::debug!(target: "yiilian_dht::udp::server", "send_to error: [index: {}]  {:?}", ctx_index, error);
                                }
                            },
                            Err(error) => {
                                log::debug!(target: "yiilian_dht::udp::server", "filter call error: [index: {}]  {:?}", ctx_index, error);
                            },
                        }
                    });
                }
            }
        }
    }
}
