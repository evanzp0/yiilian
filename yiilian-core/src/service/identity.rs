use super::Layer;
use std::fmt;

#[derive(Default, Clone)]
pub struct Identity {
    _p: (),
}

impl Identity {
    /// Create a new [`Identity`] value
    pub fn new() -> Identity {
        Identity { _p: () }
    }
}

impl<S> Layer<S> for Identity {
    type Service = S;

    fn layer(&self, inner: S) -> Self::Service {
        inner
    }
}

impl fmt::Debug for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Identity").finish()
    }
}
