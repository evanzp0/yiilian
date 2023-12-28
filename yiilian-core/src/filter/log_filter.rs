use bytes::Bytes;
use tower::Service;

use crate::data::Request;

#[derive(Debug)]
pub struct LogFilter<F> {
    ctx_index: i32,
    inner: F,
}

impl<F> LogFilter<F> {
    pub fn new(inner: F, ctx_index: i32) -> Self {
        LogFilter { inner, ctx_index }
    }
}

impl<F> Service<Request<Bytes>> for LogFilter<F> 
where
    F: Service<Request<Bytes>>
{
    type Response = F::Response;
    type Error = F::Error;
    type Future = F::Future;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Bytes>) -> Self::Future {
        log::trace!(
            target: "yiilian_core::filter::log_filter",
            "[index: {}] {:?} {} bytes, address: {}",
            self.ctx_index,
            req.dir,
            req.data.len(),
            req.remote_addr,
        );
        self.inner.call(req)
    }
}