use std::panic::{RefUnwindSafe, UnwindSafe};

use yiilian_core::{
    common::error::Error,
    data::{Body, Request, Response},
    service::service::Service,
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
            target: "yiilian_core::net",
            "recv {} bytes from address: [index: {}] {}",
            self.ctx_index,
            req.len(),
            req.remote_addr,
        );

        let rst = self.inner.call(req).await;
        match &rst {
            Ok(res) => {
                log::trace!(
                    target: "yiilian_core::net",
                    "reply {} bytes to address: [index: {}] {}",
                    self.ctx_index,
                    res.len(),
                    res.remote_addr,
                );
            }
            Err(_) => {}
        }

        rst
    }
}
