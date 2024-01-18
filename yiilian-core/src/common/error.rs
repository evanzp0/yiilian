#![allow(unused)]

use std::{error::Error as StdError, net::SocketAddr, fmt};

pub type Result<T> = std::result::Result<T, Error>;
type Cause = Box<dyn StdError + Send + Sync>;

pub struct Error {
    inner: Box<ErrorImpl>,
    description: Option<String>,
}

struct ErrorImpl {
    kind: Kind,
    cause: Option<Cause>,
    connect_info: Option<SocketAddr>,
}

#[derive(Debug)]
pub enum Kind {
    // Failure to parse bytes of a frame
    Frame,

    General,

    ChannelClosed,

    Conntrack,

    IO,

    Timeout,

    /// This error is a hack for signaling shutdown.
    /// Don't use unless you're sure you know what you're doing.
    Shutdown,

    /// Indicates that the Message type you're trying to build requires more information.
    BuilderMissingField,

    Bind,

    /// Indicates that the query token is expired or not our's token
    Token,

    /// Indicates that the transaction is handling
    Transatcion,

    /// Indicates that the deserialize file is failure
    Deserialize,

    /// Indicates that the deserialize file is failure
    Path,

    /// Indicates that the deserialize file is failure
    File,

    /// Indicates that the remote address is in the block list
    BlockList
}

impl Error {
    pub fn new(kind: Kind, description: Option<String>, cause: Option<Cause>, connect_info: Option<SocketAddr>) -> Self {
        Self {
            description,
            inner: Box::new(ErrorImpl{
                kind,
                cause,
                connect_info,
            }),
        }
    }

    pub fn new_io(cause: Option<Cause>, connect_info: Option<SocketAddr>) -> Self {
        Error::new(Kind::IO, None, cause, connect_info)
    }

    pub fn new_bind(cause: Option<Cause>) -> Self {
        Error::new(Kind::Bind, None, cause, None)
    }

    pub fn new_frame(cause: Option<Cause>, description: Option<String>,) -> Self {
        Error::new(Kind::Frame, description, cause, None)
    }

    pub fn is_timeout(&self) -> bool {
        matches!(self.inner.kind, Kind::Timeout)
    }

    pub(crate) fn find_source<E: StdError + 'static>(&self) -> Option<&E> {
        let mut cause = self.source();
        while let Some(err) = cause {
            if let Some(ref typed) = err.downcast_ref() {
                return Some(typed);
            }
            cause = err.source();
        }

        // else
        None
    }

}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_tuple("yiilian::Error");
        f.field(&self.inner.kind);
        if let Some(ref cause) = self.inner.cause {
            f.field(cause);
        }
        f.finish()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(description) = &self.description {
            f.write_str(&format!("{}", description))?
        } else {
            f.write_str(&format!("{:?}", self.inner.kind))?
        }

        if let Some(ref cause) = self.inner.cause {
            f.write_str(&format!(": {}", cause))?
        } 

        if let Some(ref connect_info) = self.inner.connect_info { 
            f.write_str(&format!(", {}", connect_info))?
        }

        Ok(())
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.inner
            .cause
            .as_ref()
            .map(|cause| &**cause as &(dyn StdError + 'static))
    }
}



// #[derive(thiserror::Error, Debug)]
// pub enum YiiLianError {
//     // Failure to parse bytes of a frame
//     #[error("Failed to parse packet bytes: {0}")]
//     FrameParse(#[source] anyhow::Error),

//     #[error("Failed to serialize msg: {0}")]
//     FrameSerialization(#[source] anyhow::Error),

//     #[error("General error: {0}")]
//     General(#[source] anyhow::Error),

//     #[error("Io channel Closed error: {0}")]
//     IoChannelClosed(#[source] anyhow::Error),

//     #[error("Transaction channel Closed error: {0}")]
//     TranChannelClosed(#[source] anyhow::Error),

//     #[error("Connection tracking error: {0}")]
//     Conntrack(#[source] anyhow::Error),

//     #[error("Socket send error: {0}")]
//     SocketSend(#[source] std::io::Error),

//     #[error("Socket recv error: {0}")]
//     SocketRecv(#[source] std::io::Error),

//     #[error("Operation timed out: {0}")]
//     Timeout(#[source] anyhow::Error),

//     /// This error is a hack for signaling shutdown.
//     /// Don't use unless you're sure you know what you're doing.
//     #[error("It's time to shutdown tasks: {0}")]
//     Shutdown(#[source] anyhow::Error),

//     /// Indicates that the Message type you're trying to build requires more information.
//     #[error("{0} is required")]
//     BuilderMissingField(&'static str),

//     /// Indicates that the builder is in an invalid/ambiguous state to build the desired
//     /// Message type.
//     #[error("Builder state invalid: {0}")]
//     BuilderInvalidCombo(&'static str),

//     #[error("Address binding  failed: {0}")]
//     Bind(#[source] std::io::Error),

//     /// Indicates that the query token is expired or not our's token
//     #[error("Token is invalid: {0}")]
//     TokenInvalid(#[source] anyhow::Error),

//     /// Indicates that the transaction is handling
//     #[error("Transactin is exists: {0}")]
//     TransatcionExist(#[source] anyhow::Error),

//     /// Indicates that the deserialize file is failure
//     #[error("Deserialize file failure: {0}")]
//     DeserializeFileFailed(#[source] anyhow::Error),

//     /// Indicates that the deserialize file is failure
//     #[error("Path operate found: {0}")]
//     PathOperateFound(#[source] anyhow::Error),

//     /// Indicates that the deserialize file is failure
//     #[error("File operate failed: {0}")]
//     FileOperateFailed(#[source] anyhow::Error),

//     /// Indicates that the remote address is in the block list
//     #[error("Interrupt by block: {0}")]
//     InterruptByBlock(#[source] anyhow::Error)
// }

// impl YiiLianError {
//     pub fn new_bind(source: std::io::Error) -> YiiLianError {
//         YiiLianError::Bind(source)
//     }

//     pub fn new_recv(source: std::io::Error) -> YiiLianError {
//         YiiLianError::SocketRecv(source)
//     }

//     pub fn new_send(source: std::io::Error) -> YiiLianError {
//         YiiLianError::SocketSend(source)
//     }

//     pub fn new_frame_parse<E>(source: anyhow::Error) -> YiiLianError {
//         YiiLianError::FrameParse(source)
//     }
// }
