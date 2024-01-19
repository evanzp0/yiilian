use std::cell::RefCell;
use std::net::ToSocketAddrs;
use std::panic::UnwindSafe;
use std::sync::Mutex;
use std::task::Poll;
use std::{net::SocketAddr, sync::Arc};

use backtrace::Backtrace;
use bytes::Bytes;
use futures::{Future, FutureExt};
use pin_project::pin_project;
use tokio::{io::ReadBuf, net::UdpSocket};
use tower::Service;

use crate::data::Body;
use crate::ready;

use crate::{
    common::error::Error,
    data::{Request, Response, UdpBody},
};

use super::io::send_to;

type Result<T> = std::result::Result<T, Error>;

thread_local! {
    static BACKTRACE: RefCell<Option<Backtrace>> = RefCell::new(None);
}

#[derive(Debug)]
#[pin_project]
pub struct Server<S> {
    pub socket: Arc<UdpSocket>,
    pub local_addr: SocketAddr,
    pub recv_filter: S,
    pub ctx_index: i32,
}

impl<S> Server<S> {
    pub fn new(ctx_index: i32, socket: Arc<UdpSocket>, recv_filter: S) -> Self {
        let local_addr = socket.local_addr().expect("Get local address error");
        Server {
            ctx_index,
            socket,
            recv_filter,
            local_addr,
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
    S: Service<Request<UdpBody>, Response = Response<UdpBody>> + Clone + Send + 'static,
    S::Error: Send + std::fmt::Debug + 'static,
    S::Future: UnwindSafe + Send + 'static,
{
    type Output = Result<()>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let me = self.project();

        std::panic::set_hook(Box::new(|_| {
            let trace = Backtrace::new();
            BACKTRACE.with(move |b| b.borrow_mut().replace(trace));
        }));

        loop {
            let mut buf = [0; 65000];
            let mut buf = ReadBuf::new(&mut buf);

            let rst = ready!(me.socket.poll_recv_from(cx, &mut buf))
                .map_err(|e| Error::new_io(Some(e.into()), me.socket.peer_addr().ok()));

            let remote_addr = match rst {
                Ok(remote_addr) => remote_addr,
                Err(error) => {
                    log::debug!(target: "yiilian_dht::udp::server", "recv error: [index: {}] {:?}", me.ctx_index, error);
                    continue;
                }
            };

            let local_addr = *me.local_addr;
            let data: Bytes = buf.filled().to_owned().into();
            let req = Request::new(UdpBody::new(data), remote_addr, local_addr);

            match me.recv_filter.poll_ready(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(_) => {
                    let filter = me.recv_filter.clone();
                    let socket = me.socket.clone();
                    let ctx_index = *me.ctx_index;

                    // 每个收到的连接都会在独立的任务中处理
                    tokio::spawn(async move {
                        // let fut = filter.call(req);
                        let filter = Mutex::new(filter);
                        let fut = {
                            let rst = std::panic::catch_unwind(|| {
                                filter.lock().unwrap().call(req)
                            });

                            match rst {
                                Ok(fut) => fut,
                                Err(error) => {
                                    // 捕获 panic 后的处理
                                    let b = BACKTRACE.with(|b| b.borrow_mut().take()).unwrap();
                                    let err_msg = panic_message::panic_message(&error);
                                    log::error!(
                                        target: "yiilian_dht::udp::server", 
                                        "filter call panic: [index: {}] {}\ntrace:\n{:?}", 
                                        ctx_index, err_msg, b);

                                    return;
                                },
                            }
                        };

                        match fut.catch_unwind().await {
                            Ok(rst) => match rst {
                                Ok(mut res) => {
                                    if let Err(error) = send_to(socket, &res.data(), res.remote_addr).await {
                                        log::debug!(target: "yiilian_dht::udp::server", "send_to error: [index: {}] {:?}", ctx_index, error);
                                    }
                                },
                                Err(error) => {
                                    log::debug!(target: "yiilian_dht::udp::server", "filter call error: [index: {}] {:?}", ctx_index, error);
                                },
                            },
                            Err(error) => {
                                // 捕获 panic 后的处理
                                let b = BACKTRACE.with(|b| b.borrow_mut().take()).unwrap();
                                let err_msg = panic_message::panic_message(&error);
                                log::error!(
                                    target: "yiilian_dht::udp::server", 
                                    "filter future panic: [index: {}] {}\ntrace:\n{:?}", 
                                    ctx_index, err_msg, b);
                            },
                        }
                    });
                }
            }
        }
    }
}
