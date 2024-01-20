use std::panic::UnwindSafe;

use futures::Future;

pub trait Service<Request> {
    /// Responses given by the service.
    type Response;

    /// Errors produced by the service.
    type Error; 

    /// Process the request and return the response asynchronously.
    fn call(&self, req: Request) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send + UnwindSafe;
}
