use core::task;
use std::future::Future;
use std::error::Error as StdError;
use std::task::Poll;

use tower::Service;

#[derive(Clone, Copy)]
pub struct MakeServiceFn<F> {
    pub f: F
}

pub fn make_service_fn<F>(f: F) -> MakeServiceFn<F> {
    MakeServiceFn {f}
}

impl<'t, F, Target, Ret, Svc, MkErr> Service<&'t Target> for MakeServiceFn<F>
where 
    F: FnMut(&Target) -> Ret, // F 是一个闭包，闭包的返回结果是一个 Future
    Ret: Future<Output = Result<Svc, MkErr>>,
    MkErr: Into<Box<dyn StdError + Send + Sync>>,
{
    type Error = MkErr;

    type Response = Svc;

    type Future = Ret;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, target: &'t Target) -> Self::Future {
        (self.f)(target)
    }
}

pub trait MakeServiceRef<Target> {
    type Error: Into<Box<dyn StdError + Send + Sync>>;
    type Service: DhtService<Error = Self::Error> + Send + Sync + 'static;
    type MakeError: Into<Box<dyn StdError + Send + Sync>> + Send + Sync + 'static;
    type Future: Future<Output = Result<Self::Service, Self::MakeError>>;

    fn poll_ready_ref(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Self::MakeError>>;

    // 调用 MakeServiceFn 中的 F 闭包，获得返回的 Future
    fn make_service_ref(&mut self, target: &Target) -> Self::Future;
}

impl<T, Target, E, ME, S, F> MakeServiceRef<Target> for T
where
    T: for<'a> Service<&'a Target, Error = ME, Response = S, Future = F>,
    E: Into<Box<dyn StdError + Send + Sync>>,
    ME: Into<Box<dyn StdError + Send + Sync>> + Send + Sync + 'static,
    S: DhtService<Error = E>  + Send + Sync + 'static,
    F: Future<Output = Result<S, ME>>,
{
    type Error = E;
    type Service = S;
    type MakeError = ME;
    type Future = F;

    fn poll_ready_ref(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Self::MakeError>> {
        self.poll_ready(cx)
    }

    fn make_service_ref(&mut self, target: &Target) -> Self::Future {
        self.call(target)
    }
}