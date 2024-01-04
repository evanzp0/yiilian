use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use bytes::Bytes;
use pin_project::pin_project;
use tokio::{io::ReadBuf, net::UdpSocket};
use tower::Service;

use crate::{
    data::{IoDir, Request},
    error::Error,
    ready,
};

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
#[pin_project]
pub struct RequestReceiver<S> {
    // channel
    pub socket: Arc<UdpSocket>,

    // service
    pub filter: S,

    // 重试次数
    pub retry_times: u8,

    ctx_index: i32,
}

impl<S> RequestReceiver<S> {
    pub fn new(socket: Arc<UdpSocket>, filter: S, ctx_index: i32) -> Self {
        let retry_times = 0;
        RequestReceiver { socket, filter, retry_times, ctx_index}
    }
}

impl<S> Future for RequestReceiver<S>
where
    S: Service<Request<Bytes>>,
    S::Future: Send + 'static,
    S::Response: Send + 'static,
    S::Error: Send + 'static,
{
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
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
                },
                Err(error) => {
                    if *me.retry_times >= 3 {
                        log::error!(target: "yiilian_dht::udp::RequestReceiver", "[index: {}] {:?}", *me.ctx_index, error); 
                        return Poll::Ready(Err(error))
                    } else {
                        *me.retry_times += 1;
                        continue;
                    }
                },
            };

            let local_addr = me
                .socket
                .local_addr()
                .map_err(|e| Error::new_io(Some(e.into()), me.socket.local_addr().ok()))?;
    
            let data: Bytes = buf.filled().to_owned().into();
            let req = Request::new(data, remote_addr, local_addr, IoDir::Recv);
    
            let fut = me.filter.call(req);
    
            // 每个收到的连接都会在独立的任务中处理
            tokio::spawn(fut);
        }
    }
}
