use std::sync::Arc;

use yiilian_core::data::Request;

use crate::{common::context::Context, data::body::KrpcBody};

#[derive(Clone)]
pub enum Event {
    RecvRequest(EventBody<Arc<Request<KrpcBody>>>),
}

impl Event {
    pub fn get_type(&self) -> &str {
        match self {
            Event::RecvRequest(_) => "recv_request",
        }
    }
}

#[derive(Clone)]
pub struct EventBody<T> {
    pub ctx: Arc<Context>,
    pub data: T
}