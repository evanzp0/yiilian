use std::panic::{RefUnwindSafe, UnwindSafe};

use yiilian_core::{
    common::error::Error,
    data::{Body, Request, Response},
    service::{Service, Layer},
};

#[derive(Debug, Clone)]
pub struct LogService<S> {
    ctx_index: i32,
    inner: S,
}

impl<S> LogService<S> {
    pub fn new(ctx_index: i32, inner: S) -> Self {
        LogService { inner, ctx_index }
    }
}

impl<S, B> Service<Request<B>> for LogService<S>
where
    S: Service<Request<B>, Response = Response<B>, Error = Error> + Send + Sync + RefUnwindSafe,
    B: Body + Send + UnwindSafe,
{
    type Response = S::Response;
    type Error = S::Error;

    async fn call(&self, req: Request<B>) -> Result<Self::Response, Self::Error> {
        // if req.len() == 3 {
        //     panic!("req.len == 3")
        // }
        log::trace!(
            target: "yiilian_raw::net",
            "recv {} bytes from address: [index: {}] {}",
            req.len(),
            self.ctx_index,
            req.remote_addr,
        );

        let rst = self.inner.call(req).await;
        match &rst {
            Ok(res) => {
                log::trace!(
                    target: "yiilian_raw::net",
                    "reply {} bytes to address: [index: {}] {}",
                    res.len(),
                    self.ctx_index,
                    res.remote_addr,
                );
            }
            Err(_) => {}
        }

        rst
    }
}

pub struct LogLayer{
    ctx_index: i32
}

impl LogLayer {
    pub fn new(ctx_index: i32) -> Self {
        LogLayer { ctx_index }
    }
}

impl<S> Layer<S> for LogLayer {
    type Service = LogService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LogService::new(self.ctx_index, inner)
    }
}