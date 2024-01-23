use std::net::SocketAddr;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use tokio::sync::oneshot;
use yiilian_core::common::util::random_bytes;

use crate::{common::id::Id, data::body::{Query, Reply}};

#[derive(Debug)]
/// 对外发送请求时，需要记录事务，当对方有 feedback 时，需要将该事务核销
pub struct Transaction {
    pub(crate) id: TransactionId,
    pub(crate) node_id: Option<Id>,
    pub(crate) addr: SocketAddr,
    pub(crate) message: Query,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) response_channel: Option<oneshot::Sender<Reply>>,
}

impl Transaction {
    pub fn new(
        id: TransactionId,
        node_id: Option<Id>,
        addr: SocketAddr,
        message: Query,
        response_channel: Option<oneshot::Sender<Reply>>,
    ) -> Self {
        Transaction {
            id,
            node_id,
            addr,
            message,
            created_at: Utc::now(),
            response_channel,
        }
    }

    pub fn get_id(&self) -> &TransactionId {
        &self.id
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
/// Represents a DHT transaction id, which are basically just small byte strings.
/// This type is not yet used widely across this codebase.
pub struct TransactionId(pub(crate) Bytes);

impl From<Bytes> for TransactionId {
    fn from(tid: Bytes) -> Self {
        TransactionId(tid)
    }
}

impl From<&str> for TransactionId {
    fn from(tid: &str) -> Self {
        TransactionId(tid.as_bytes().to_owned().into())
    }
}

impl TransactionId {
    pub fn from_random() -> Self {
        TransactionId(random_bytes(2).into())
    }

    pub fn get_bytes(&self) -> Bytes {
        self.0.clone()
    }
}