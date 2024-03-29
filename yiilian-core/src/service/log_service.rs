
use std::error::Error as StdError;

use crate::{
    data::{Body, Request, Response},
    service::{Service, Layer},
};

#[derive(Debug, Clone)]
pub struct LogService<S> {
    inner: S,
}

impl<S> LogService<S> {
    pub fn new(inner: S) -> Self {
        LogService { inner }
    }
}

impl<S, B1, B2> Service<Request<B1>> for LogService<S>
where
    S: Service<Request<B1>, Response = Response<B2>> + Send + Sync,
    S::Error: Into<Box<dyn StdError + Send + Sync>>,
    B1: Body + Send,
    B2: Body + Send,
{
    type Response = S::Response;
    type Error = S::Error;

    async fn call(&mut self, req: Request<B1>) -> Result<Self::Response, Self::Error> {
        let local_port = req.local_addr.port();
        log::trace!(
            target: "yiilian_core::service::log_service",
            "[{}] recv {} bytes from address: {}",
            req.local_addr.port(),
            req.len(),
            req.remote_addr,
        );

        let rst = self.inner.call(req).await;
        match &rst {
            Ok(res) => {
                log::trace!(
                    target: "yiilian_core::service::log_service",
                    "[{}] reply {} bytes to address:  {}",
                    local_port,
                    res.len(),
                    res.remote_addr,
                );
            }
            Err(_) => {}
        }

        rst
    }
}

pub struct LogLayer;

impl<S> Layer<S> for LogLayer {
    type Service = LogService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LogService::new(inner)
    }
}