use bytes::Bytes;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InMessage(pub Bytes);