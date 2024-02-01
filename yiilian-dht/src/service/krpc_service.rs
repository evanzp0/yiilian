
use std::error::Error as StdError;

use futures::Future;

use yiilian_core::{
    data::{Request, Response, Body},
    service::Service,
};

pub trait KrpcService<ReqBody> {

    type ResBody: Body;

    type Error: Into<Box<dyn StdError + Send + Sync>>;

    fn call(&mut self, req: Request<ReqBody>) -> impl Future<Output = Result<Response<Self::ResBody>, Self::Error>> + Send;
}

impl<T, B1, B2> KrpcService<B1> for T
where
    T: Service<Request<B1>, Response = Response<B2>> + Send + Sync,
    B1: Send,
    B2: Body,
    T::Error: Into<Box<dyn StdError + Send + Sync + 'static>>,
{
    type ResBody = B2;
    type Error = T::Error;

    async fn call(&mut self, req: Request<B1>) -> Result<Response<Self::ResBody>, Self::Error> {
        Service::call(self, req).await
    }
}