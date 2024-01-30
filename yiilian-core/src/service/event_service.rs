use std::panic::{RefUnwindSafe, UnwindSafe};
use std::error::Error as StdError;

use crate::{
    data::{Body, Request, Response},
    service::{Service, Layer},
};

#[derive(Debug, Clone)]
pub struct EventService<S> {
    inner: S,
}

impl<S> EventService<S> {
    pub fn new(inner: S) -> Self {
        EventService { inner }
    }
}

impl<S, B1, B2> Service<Request<B1>> for EventService<S>
where
    S: Service<Request<B1>, Response = Response<B2>> + Send + Sync + RefUnwindSafe,
    S::Error: Into<Box<dyn StdError + Send + Sync>>,
    B1: Body + Send + UnwindSafe,
    B2: Body + Send + UnwindSafe,
{
    type Response = S::Response;
    type Error = S::Error;

    async fn call(&self, req: Request<B1>) -> Result<Self::Response, Self::Error> {
        todo!()
    }
}

pub struct EventLayer;

impl<S> Layer<S> for EventLayer {
    type Service = EventService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        EventService::new(inner)
    }
}