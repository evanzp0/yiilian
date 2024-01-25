use std::future::Future;
use std::error::Error as StdError;
use std::panic::UnwindSafe;
use std::sync::Arc;
use yiilian_core::service::Service;

#[derive(Clone, Copy)]
pub struct MakeServiceFn<F> {
    pub f: F
}

pub fn make_service_fn<F>(f: F) -> MakeServiceFn<F> {
    MakeServiceFn {f}
}

impl<F, Target, Ret, Svc, MkErr> Service<Arc<Target>> for MakeServiceFn<F>
where 
    Target: 'static,
    F: Fn(Arc<Target>) -> Ret, // F 是一个闭包，闭包的返回结果是一个 Future
    Ret: Future<Output = Result<Svc, MkErr>> + Send + Sync + UnwindSafe,
    MkErr: Into<Box<dyn StdError + Send + Sync>>,
{
    type Error = MkErr;

    type Response = Svc;

    fn call(&self, target: Arc<Target>) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send + UnwindSafe {
        (self.f)(target)
    }
}

