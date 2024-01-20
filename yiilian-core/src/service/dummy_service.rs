
use std::task::Poll;

use futures::future::{Ready, ready};
use tower::Service;

use crate::{data::{Request, Response}, common::error::Error};

#[derive(Debug, Clone)]
pub struct DummyService;

impl DummyService {
    pub fn new() -> Self {
        DummyService
    }
}

impl<B> Service<Request<B>> for DummyService
{
    type Response = Response<B>;
    type Error = Error;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        ready(Ok(Response::new(req.body, req.remote_addr, req.local_addr)))
    }
}
