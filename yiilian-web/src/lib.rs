
pub mod common;
pub mod handle;

pub const STATIC_DIR: &str = "static";

use common::WebError;

pub type Result<T> = std::result::Result<T, WebError>;