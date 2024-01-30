use std::panic::{RefUnwindSafe, UnwindSafe};
use std::{error::Error as StdError, fmt::Debug};

use tokio::sync::broadcast::Sender;

use crate::{
    data::{Body, Request, Response},
    service::{Service, Layer},
};

#[derive(Debug, Clone)]
pub struct EventService<S, T> {
    inner: S,
    tx: Sender<T>
}

impl<S, T> EventService<S, T> {
    pub fn new(inner: S, tx: Sender<T>) -> Self {
        EventService { inner, tx }
    }
}

impl<S, T> RefUnwindSafe for EventService<S, T> {}

impl<S, B1, B2> Service<Request<B1>> for EventService<S, Request<B1>>
where
    S: Service<Request<B1>, Response = Response<B2>> + Send + Sync + RefUnwindSafe,
    S::Error: Into<Box<dyn StdError + Send + Sync>>,
    B1: Clone + Debug + Body + Send + UnwindSafe,
    B2: Body + Send + UnwindSafe,
{
    type Response = S::Response;
    type Error = S::Error;

    async fn call(&self, req: Request<B1>) -> Result<Self::Response, Self::Error> {
        if let Err(e) = self.tx.send(req.clone()) {
            log::debug!("error: {:?}", e);
        }

        self.inner.call(req).await
    }
}

pub struct EventLayer<T> {
    tx: Sender<T>
}

impl<T> EventLayer<T> {
    pub fn new(tx: Sender<T>) -> Self {
        Self {
            tx
        }
    }
}

impl<S, T> Layer<S> for EventLayer<T> {
    type Service = EventService<S, T>;

    fn layer(&self, inner: S) -> Self::Service {
        EventService::new(inner, self.tx.clone())
    }
}