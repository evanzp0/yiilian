use crate::{data::{Request, Response, RawBody}, common::error::Error};

use super::service::Service;

#[derive(Debug, Clone)]
pub struct DummyService;

impl DummyService {
    pub fn new() -> Self {
        DummyService
    }
}

impl Service<Request<RawBody>> for DummyService
{
    type Response = Response<RawBody>;
    type Error = Error;

    async fn call(&self, req: Request<RawBody>) -> Result<Self::Response, Self::Error> {
        Ok(Response::new(req.body, req.remote_addr, req.local_addr))
    }
}
