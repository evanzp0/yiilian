
use crate::{
    common::error::Error, data::{Request, Response}, service::Service
};

#[derive(Clone)]
pub struct TestService;

#[allow(unused)]
impl TestService {
    pub fn new() -> Self {
        TestService
    }
}

impl Service<Request<i32>> for TestService
{
    type Response = Response<i32>;
    type Error = Error;

    async fn call(&mut self, req: Request<i32>) -> Result<Self::Response, Self::Error> {
        Ok(Response::new(req.body, req.remote_addr, req.local_addr))
    }
}
