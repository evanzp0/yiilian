use std::error::Error as StdError;
use std::fmt;
use std::future::Future;
use std::marker::PhantomData;

use crate::data::{Request, Response, Body};
use crate::service::service::Service;

/// Create a `Service` from a function.
pub fn service_fn<F, R, S>(f: F) -> ServiceFn<F, R>
where
    F: Fn(Request<R>) -> S,
    S: Future,
{
    ServiceFn {
        f,
        _req: PhantomData,
    }
}

/// Service returned by [`service_fn`]
pub struct ServiceFn<F, R> {
    f: F,
    _req: PhantomData<fn(R)>,
}

impl<F, ReqBody, Ret, ResBody, E> Service<Request<ReqBody>> for ServiceFn<F, ReqBody>
where
    F: Fn(Request<ReqBody>) -> Ret + Sync + Send,
    ReqBody: Body + Send,
    Ret: Future<Output = Result<Response<ResBody>, E>> + Send,
    E: Into<Box<dyn StdError + Send + Sync>>,
    ResBody: Body + Send,
{
    type Response = Response<ResBody>;
    type Error = E;
    
    async fn call(&mut self, req: Request<ReqBody>) -> Result<Self::Response, Self::Error> {
        (self.f)(req).await
    }
}

impl<F, R> fmt::Debug for ServiceFn<F, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("impl Service").finish()
    }
}

impl<F, R> Clone for ServiceFn<F, R>
where
    F: Clone,
{
    fn clone(&self) -> Self {
        ServiceFn {
            f: self.f.clone(),
            _req: PhantomData,
        }
    }
}

impl<F, R> Copy for ServiceFn<F, R> where F: Copy {}
