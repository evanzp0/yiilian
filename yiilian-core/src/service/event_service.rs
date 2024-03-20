
use std::{error::Error as StdError, fmt::Debug, sync::Arc};

use tokio::sync::broadcast::Sender;

use crate::{
    data::{Body, Request, Response},
    service::{Service, Layer},
};

#[derive(Debug, Clone)]
pub struct EventService<S, T> {
    inner: S,
    tx: Sender<Arc<T>>
}

impl<S, T> EventService<S, T> {
    pub fn new(inner: S, tx: Sender<Arc<T>>) -> Self {
        EventService { inner, tx }
    }
}


impl<S, B1, B2> Service<Request<B1>> for EventService<S, Request<B1>>
where
    S: Service<Request<B1>, Response = Response<B2>> + Send + Sync,
    S::Error: Into<Box<dyn StdError + Send + Sync>>,
    B1: Clone + Debug + Body + Send + Sync,
    B2: Body + Send,
{
    type Response = S::Response;
    type Error = S::Error;

    async fn call(&mut self, req: Request<B1>) -> Result<Self::Response, Self::Error> {

        let rst = self.inner.call(req.clone()).await;

        match rst {
            Ok(reply) => {
                let req = Arc::new(req);
                
                if let Err(e) = self.tx.send(req) {
                    log::debug!("error: {:?}", e);
                }

                Ok(reply)
            },
            Err(error) => Err(error),
        }
    }
}

pub struct EventLayer<T> {
    tx: Sender<Arc<T>>
}

impl<T> EventLayer<T> {
    pub fn new(tx: Sender<Arc<T>>) -> Self {
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