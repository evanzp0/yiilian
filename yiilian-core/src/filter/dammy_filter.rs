
use std::{task::Poll, convert::Infallible};

use futures::future::{Ready, ready};
use tower::Service;

use crate::data::{Request, Response};

#[derive(Debug)]
pub struct DammyFilter;

impl DammyFilter {
    pub fn new() -> Self {
        DammyFilter
    }
}

impl Service<Request> for DammyFilter
{
    type Response = Response;
    type Error = Infallible;
    type Future = Ready<Result<Response, Infallible>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        ready(Ok(Response::new(req.data, req.remote_addr, req.local_addr)))
    }
}
