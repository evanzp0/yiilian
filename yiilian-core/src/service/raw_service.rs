
use std::{error::Error as StdError, panic::{UnwindSafe, RefUnwindSafe}};

use futures::Future;

use crate::data::{Request, Body, Response};

use super::service::Service;

pub trait RawService<ReqBody> {

    type ResBody: Body;

    type Error: Into<Box<dyn StdError + Send + Sync>>;

    fn call(&self, req: Request<ReqBody>) -> impl Future<Output = Result<Response<Self::ResBody>, Self::Error>> + Send + UnwindSafe;
}

impl<T, B1, B2> RawService<B1> for T
where
    T: Service<Request<B1>, Response = Response<B2>> + Send + Sync + RefUnwindSafe,
    B1: Send + UnwindSafe,
    B2: Body + UnwindSafe,
    T::Error: Into<Box<dyn StdError + Send + Sync + 'static>>,
{
    type ResBody = B2;
    type Error = T::Error;

    async fn call(&self, req: Request<B1>) -> Result<Response<Self::ResBody>, Self::Error> {
        Service::call(self, req).await
    }
}

// pub trait RawService<ReqBody> {

//     type ResBody: Body;

//     type Error: Into<Box<dyn StdError + Send + Sync>>;

//     fn call(&self, req: String) -> impl Future<Output = Result<Self::ResBody, Self::Error>> + Send;
// }

// impl<T> RawService<String> for T
// where
//     T: Service<String, Response = String> + Send + Sync,
//     T::Error: Into<Box<dyn StdError + Send + Sync>>,
// {
//     type ResBody = String;
//     type Error = T::Error;

//     async fn call(&self, req: String) -> Result<Self::ResBody, Self::Error> {
//         Service::call(self, req).await
//     }
// }