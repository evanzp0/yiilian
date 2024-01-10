use core::task;
use std::{future::Future, task::Poll};

use std::error::Error as StdError;
use tower::Service;

use crate::data::Request;

pub trait DhtService {

    type Error: Into<Box<dyn StdError + Send + Sync>>;

    /// The `Future` returned by this `Service`.
    type Future: Future<Output = Result<(), Self::Error>> + Send + Sync;

    fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>>;

    fn call(&mut self, req: Request) -> Self::Future;
}

impl<T> DhtService for T
where
    T: Service<Request, Response = ()>,
    T::Error: Into<Box<dyn StdError + Send + Sync>>,
    T:: Future: Send + Sync,
{
    type Error = T::Error;
    type Future = T::Future;

    fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Service::poll_ready(self, cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        Service::call(self, req)
    }
}