use std::sync::Arc;

pub trait Layer<S> {
    /// The wrapped service
    type Service;
    /// Wrap the given service with the middleware, returning a new service
    /// that has been decorated with the middleware.
    fn layer(&self, inner: S) -> Self::Service;
}

impl<'a, T, S> Layer<S> for &'a T
where
    T: ?Sized + Layer<S>,
{
    type Service = T::Service;

    fn layer(&self, inner: S) -> Self::Service {
        (**self).layer(inner)
    }
}

impl<'a, T, S> Layer<S> for Arc<T>
where
    T: ?Sized + Layer<S>,
{
    type Service = T::Service;

    fn layer(&self, inner: S) -> Self::Service {
        (**self).layer(inner)
    }
}

impl<'a, T, S> Layer<S> for Box<T>
where
    T: ?Sized + Layer<S>,
{
    type Service = T::Service;

    fn layer(&self, inner: S) -> Self::Service {
        (**self).layer(inner)
    }
}
