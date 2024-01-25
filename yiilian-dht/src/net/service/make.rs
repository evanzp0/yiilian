use std::future::Future;
use std::error::Error as StdError;
use std::panic::UnwindSafe;
use std::sync::Arc;
use yiilian_core::data::Body;
use yiilian_core::service::Service;

use super::KrpcService;

#[derive(Clone, Copy)]
pub struct MakeServiceFn<F> {
    pub f: F
}

pub fn make_service_fn<F>(f: F) -> MakeServiceFn<F> {
    MakeServiceFn {f}
}

impl<F, Target, Ret, Svc, MkErr> Service<Arc<Target>> for MakeServiceFn<F>
where
    F: Fn(Arc<Target>) -> Ret, // F 是一个闭包，闭包的返回结果是一个 Future
    Ret: Future<Output = Result<Svc, MkErr>> + Send + UnwindSafe,
    MkErr: Into<Box<dyn StdError + Send + Sync>>,
{
    type Error = MkErr;

    type Response = Svc;

    fn call(&self, target: Arc<Target>) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send + UnwindSafe {
        (self.f)(target)
    }
}

// Just a sort-of "trait alias" of `MakeService`, not to be implemented
// by anyone, only used as bounds.
pub trait MakeServiceRef<Target, ReqBody> {
    type ResBody: Body;
    type Error: Into<Box<dyn StdError + Send + Sync>>;
    type Service: KrpcService<ReqBody, ResBody = Self::ResBody, Error = Self::Error>;

    fn make_service_ref(&self, target: Arc<Target>) -> impl Future<Output = Result<Self::Service, Self::Error>>;
}

impl<T, Target, E, S, IB, OB> MakeServiceRef<Target, IB> for T
where
    T: Service<Arc<Target>, Response = S, Error = E>,
    E: Into<Box<dyn StdError + Send + Sync>>,
    S: KrpcService<IB, ResBody = OB, Error = E>,
    IB: Body,
    OB: Body,
{
    type Error = E;
    type Service = S;
    type ResBody = OB;

    fn make_service_ref(&self, target: Arc<Target>) -> impl Future<Output = Result<Self::Service, Self::Error>> {
        self.call(target)
    }
}
