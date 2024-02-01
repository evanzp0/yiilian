
use std::error::Error as StdError;

use yiilian_core::{
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

impl<S, B> Service<Request<B>> for LogService<S>
where
    S: Service<Request<B>, Response = Response<B>> + Send + Sync,
    S::Error: Into<Box<dyn StdError + Send + Sync>>,
    B: Body + Send,
{
    type Response = S::Response;
    type Error = S::Error;

    async fn call(&mut self, req: Request<B>) -> Result<Self::Response, Self::Error> {
        // if req.len() == 3 {
        //     panic!("req.len == 3")
        // }
        let local_port = req.local_addr.port();
        log::trace!(
            target: "yiilian_raw::net",
            "recv {} bytes from address: [{}] {}",
            req.len(),
            req.local_addr.port(),
            req.remote_addr,
        );

        let rst = self.inner.call(req).await;
        match &rst {
            Ok(res) => {
                log::trace!(
                    target: "yiilian_raw::net",
                    "reply {} bytes to address: [{}] {}",
                    res.len(),
                    local_port,
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