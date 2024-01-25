use std::sync::Arc;

use yiilian_core::{
    common::error::Error,
    data::{Request, Response},
    service::Service,
};

use crate::{common::context::Context, data::body::KrpcBody};

#[allow(unused)]
#[derive(Clone)]
pub struct DummyService {
    ctx: Arc<Context>,
}

impl DummyService {
    pub fn new(ctx: Arc<Context>) -> Self {
        DummyService {
            ctx
        }
    }
}

impl Service<Request<KrpcBody>> for DummyService {
    type Response = Response<KrpcBody>;
    type Error = Error;

    async fn call(&self, req: Request<KrpcBody>) -> Result<Self::Response, Self::Error> {
        Ok(Response::new(req.body, req.remote_addr, req.local_addr))
    }
}
