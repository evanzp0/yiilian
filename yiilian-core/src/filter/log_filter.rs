
use std::{future::Future, task::{Poll, Context}};

use pin_project::pin_project;
use tower::Service;

use crate::{data::{Request, Response, Body}, ready, common::error::Error};

#[derive(Debug, Clone)]
pub struct LogFilter<F> {
    ctx_index: i32,
    inner: F,
}

impl<F> LogFilter<F> {
    pub fn new(inner: F, ctx_index: i32) -> Self {
        LogFilter { inner, ctx_index }
    }
}

impl<F, B> Service<Request<B>> for LogFilter<F> 
where
    F: Service<Request<B>, Response = Response<B>, Error = Error>,
    B: Body,
{
    type Response = F::Response;
    type Error = F::Error;
    type Future = LogFuture<F::Future>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        log::trace!(
            target: "yiilian_core::filter::log_filter",
            "[index: {}] recv {} bytes, address: {}",
            self.ctx_index,
            req.len(),
            req.remote_addr,
        );

        let fut = self.inner.call(req);

        LogFuture {
            fut,
            ctx_index: self.ctx_index,
        }
    }
}

#[pin_project]
pub struct LogFuture<F> {
    #[pin]
    fut: F,
    ctx_index: i32,
}

impl<F, B> Future for LogFuture<F> 
where
    F: Future<Output = Result<Response<B>, Error>>,
    B: Body,
{
    type Output = Result<Response<B>, Error>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.project();

        match ready!(me.fut.poll(cx)) {
            Ok(res) => {
                log::trace!(
                    target: "yiilian_core::filter::log_filter",
                    "[index: {}] reply {} bytes, address: {}",
                    me.ctx_index,
                    res.len(),
                    res.remote_addr,
                );
                Poll::Ready(Ok(res))
            },
            Err(e) => {
                log::trace!(
                    target: "yiilian_core::filter::log_filter",
                    "[index: {}] error: {}",
                    me.ctx_index,
                    e
                );
                Poll::Ready(Err(e))
            },
        }
    }
}