use core::task;
use std::{future::Future, task::Poll};

use std::error::Error as StdError;
use tower::Service;

use crate::data::{Request, Body, Response};

pub trait DhtService<ReqBody> {

    type ResBody: Body;

    type Error: Into<Box<dyn StdError + Send + Sync>>;

    /// The `Future` returned by this `Service`.
    type Future: Future<Output = Result<(), Self::Error>> + Send + Sync;

    fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>>;

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future;
}

impl<T, B1, B2> DhtService<B1> for T
where
    T: Service<Request<B2>, Response = Response<B2>>,
    T::Error: Into<Box<dyn StdError + Send + Sync>>,
    T:: Future: Send + Sync,
{
    type ResBody = B2;
    type Error = T::Error;
    type Future = T::Future;

    fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Service::poll_ready(self, cx)
    }

    fn call(&mut self, req: Request<B1>) -> Self::Future {
        Service::call(self, req)
    }
}