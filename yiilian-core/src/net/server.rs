use std::fmt::Debug;
use std::net::ToSocketAddrs;
use std::{net::SocketAddr, sync::Arc};

use bytes::Bytes;
use futures::FutureExt;
use pin_project::pin_project;
use tokio::net::UdpSocket;

use crate::common::error::trace_panic;
use crate::data::Body;

use crate::service::raw_service::RawService;
use crate::{
    common::error::Error,
    data::{RawBody, Request},
};

use super::io::send_to;

type Result<T> = std::result::Result<T, Error>;

// thread_local! {
//     static BACKTRACE: RefCell<Option<Backtrace>> = RefCell::new(None);
// }

#[derive(Debug)]
#[pin_project]
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
                    log::debug!(target: "yiilian_dht::net::server", "recv error: [index: {}] {:?}", self.ctx_index, error);
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
                            if let Err(error) = send_to(socket, &res.data(), res.remote_addr).await
                            {
                                log::debug!(target: "yiilian_dht::net::server", "send_to error: [index: {}] {:?}", ctx_index, error);
                            }
                        }
                        Err(error) => {
                            log::debug!(target: "yiilian_dht::net::server", "filter call error: [index: {}] {:?}", ctx_index, error);
                        }
                    },
                    Err(error) => {
                        // 捕获 panic 后的处理
                        let (b, err) = trace_panic(&error);
                        log::error!(
                                    target: "yiilian_dht::net::server",
                                    "filter future panic: [index: {}] {}\ntrace:\n{:?}",
                                    ctx_index, err, b);
                    }
                }
            });
        }
    }
}

// impl<S> Future for Server<S>
// where
//     S: Service<Request<UdpBody>, Response = Response<UdpBody>> + Clone + Send + 'static,
//     S::Error: Send + std::fmt::Debug + 'static,
//     S::Future: UnwindSafe + Send + 'static,
// {
//     type Output = Result<()>;

//     fn poll(
//         self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Self::Output> {
//         let me = self.project();

//         // std::panic::set_hook(Box::new(|_| {
//         //     let trace = Backtrace::new();
//         //     BACKTRACE.with(move |b| b.borrow_mut().replace(trace));
//         // }));

//         loop {
//             let mut buf = [0; 65000];
//             let mut buf = ReadBuf::new(&mut buf);

//             let rst = ready!(me.socket.poll_recv_from(cx, &mut buf))
//                 .map_err(|e| Error::new_io(Some(e.into()), me.socket.local_addr().ok()));

//             let remote_addr = match rst {
//                 Ok(remote_addr) => remote_addr,
//                 Err(error) => {
//                     log::debug!(target: "yiilian_dht::udp::server", "recv error: [index: {}] {:?}", me.ctx_index, error);
//                     continue;
//                 }
//             };

//             let local_addr = *me.local_addr;
//             let data: Bytes = buf.filled().to_owned().into();
//             let req = Request::new(UdpBody::new(data), remote_addr, local_addr);

//             match me.recv_service.poll_ready(cx) {
//                 Poll::Pending => return Poll::Pending,
//                 Poll::Ready(_) => {
//                     let filter = me.recv_service.clone();
//                     let socket = me.socket.clone();
//                     let ctx_index = *me.ctx_index;

//                     // 每个收到的连接都会在独立的任务中处理
//                     tokio::spawn(async move {
//                         // let fut = filter.call(req);
//                         let filter = Mutex::new(filter);
//                         let fut = {
//                             let rst = std::panic::catch_unwind(|| {
//                                 filter.lock().unwrap().call(req)
//                             });

//                             match rst {
//                                 Ok(fut) => fut,
//                                 Err(error) => {
//                                     // 捕获 panic 后的处理
//                                     let (b, err) = trace_panic(&error);
//                                     log::error!(
//                                         target: "yiilian_dht::udp::server",
//                                         "filter call panic: [index: {}] {}\ntrace:\n{:?}",
//                                         ctx_index, err, b);

//                                     return;
//                                 },
//                             }
//                         };

//                         match fut.catch_unwind().await {
//                             Ok(rst) => match rst {
//                                 Ok(mut res) => {
//                                     if let Err(error) = send_to(socket, &res.data(), res.remote_addr).await {
//                                         log::debug!(target: "yiilian_dht::udp::server", "send_to error: [index: {}] {:?}", ctx_index, error);
//                                     }
//                                 },
//                                 Err(error) => {
//                                     log::debug!(target: "yiilian_dht::udp::server", "filter call error: [index: {}] {:?}", ctx_index, error);
//                                 },
//                             },
//                             Err(error) => {
//                                 // 捕获 panic 后的处理
//                                 let (b, err) = trace_panic(&error);
//                                 log::error!(
//                                     target: "yiilian_dht::udp::server",
//                                     "filter future panic: [index: {}] {}\ntrace:\n{:?}",
//                                     ctx_index, err, b);
//                             },
//                         }
//                     });
//                 }
//             }
//         }
//     }
// }
