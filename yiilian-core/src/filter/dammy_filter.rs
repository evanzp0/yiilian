
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

impl<B> Service<Request<B>> for DammyFilter
{
    type Response = Response<B>;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Infallible>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        ready(Ok(Response::new(req.body, req.remote_addr, req.local_addr)))
    }
}
