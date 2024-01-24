use std::sync::Arc;

use yiilian_core::data::Request;

use crate::data::body::KrpcBody;

#[derive(Debug, Clone)]
pub enum Event {
    RecvRequest(Arc<Request<KrpcBody>>),
}

impl Event {
    pub fn get_type(&self) -> &str {
        match self {
            Event::RecvRequest(_) => "recv_request",
        }
    }
}