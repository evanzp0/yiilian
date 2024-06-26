pub mod error;
pub mod util;
pub mod shutdown;
pub mod expect_log;
pub mod working_dir;

#[macro_export]
macro_rules! ready {
    ($e:expr) => {
        match $e {
            std::task::Poll::Ready(v) => v,
            std::task::Poll::Pending => return std::task::Poll::Pending,
        }
    };
}