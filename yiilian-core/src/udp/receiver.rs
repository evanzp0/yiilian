use std::{sync::Mutex, future::Future, pin::Pin, result, task::{Poll, Context}};

use pin_project::pin_project;
use tokio::sync::mpsc::Receiver;
use anyhow::anyhow;
use tower::Service;

use crate::{error::YiiLianError, data::Request};

type Result<T> = std::result::Result<T, YiiLianError>;

#[derive(Debug)]
#[pin_project]
pub struct RequestReceiver<S, B> 
{   
    // channel
    pub inner: Mutex<Receiver<Request<B>>>,

    // service
    pub filter: S,
}

impl<B> RequestReceiver<(), B> {
    pub fn builder(receiver: Receiver<Request<B>>) -> RequestReceiverBuilder<B> {
        RequestReceiverBuilder {
            inner: receiver
        }
    }

    pub fn bind(receiver: Receiver<Request<B>>) -> RequestReceiverBuilder<B> {
        RequestReceiver::builder(receiver)
    }
}

impl<S, B> Future for RequestReceiver<S, B> 
where
    S: Service<Request<B>>,
    S::Future: Send + 'static,
    S::Response: Send + 'static,
    S::Error: Send + 'static,
{
    type Output = Result<Option<Request<B>>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.project();
        let rst = me.inner.lock().unwrap().poll_recv(cx);

        match rst {
            Poll::Ready(result) => match result {
                Some(req) => {
                    let fut = me.filter.call(req);
                    let mut handle = tokio::spawn(fut);

                    let handle = Pin::new(&mut handle);

                    todo!()
                //    return Poll::Ready(Ok(Some(req)))
                },
                None => todo!(),
            },
            Poll::Pending => return Poll::Pending,
        }
        

        todo!()
    }
}

// impl<F> RequestReceiver<F, B>
// where
//     F: Service<UdpPacket, Response = Option<UdpPacket>>,
// {
//     pub fn recv(&self) -> Result<Option<UdpPacket>> {
//         let packet = self.inner.lock().unwrap().recv().await;
//         match packet {
//             Some(packet) => {
//                 self.filter.call(packet).await
//             },
//             None => 
//                 Err(YiiLianError::IoChannelClosed(anyhow!("recv_from_rx channel in inner recv loop closed"))),
//         }
//     }
// }

#[derive(Debug)]
pub struct RequestReceiverBuilder<B> {
    inner: Receiver<Request<B>>,
}

// impl RequestReceiverBuilder {
//     pub fn serve<F>(self, filter:F) -> RequestReceiver<F>
//     where
//         F: Filter<UdpPacket>,
//     {
//         RequestReceiver {
//             inner: Mutex::new(self.inner),
//             filter: filter,
//         }
//     }
// }
