use yiilian_core::{
    common::error::Error,
    data::{Request, Response},
    service::Service,
};

use crate::data::raw_body::RawBody;

#[derive(Debug, Clone)]
pub struct DummyService;

impl DummyService {
    pub fn new() -> Self {
        DummyService
    }
}

impl Service<Request<RawBody>> for DummyService {
    type Response = Response<RawBody>;
    type Error = Error;

    async fn call(&mut self, req: Request<RawBody>) -> Result<Self::Response, Self::Error> {
        Ok(Response::new(req.body, req.remote_addr, req.local_addr))
    }
}
