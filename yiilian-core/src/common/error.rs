#![allow(unused)]

use std::{error::Error as StdError, net::SocketAddr, fmt, cell::RefCell, any::Any};

use backtrace::Backtrace;

pub type Result<T> = std::result::Result<T, Error>;
type Cause = Box<dyn StdError + Send + Sync>;

thread_local! {
    pub static BACKTRACE: RefCell<Option<Backtrace>> = RefCell::new(None);
}

pub struct Error {
    inner: Box<ErrorImpl>,
    description: Option<String>,
}

struct ErrorImpl {
    kind: Kind,
    cause: Option<Cause>,
    connect_info: Option<SocketAddr>,
}

#[derive(Debug, Clone, Copy)]
pub enum Kind {
    // Failure to parse bytes of a frame
    Frame,

    Id,

    General,

    ChannelClosed,

    /// Indicates connect track error
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
    Block,

    /// Indicates Network error
    Net,

    /// Indicates Decode error
    Decode,

    /// Indicates Memory error
    Memory,
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

    pub fn new_memory(cause: Option<Cause>, description: Option<String>) ->Self {
        Error::new(Kind::Memory, description, cause, None)
    } 

    pub fn new_conntrack(cause: Option<Cause>, description: Option<String>, connect_info: Option<SocketAddr>) -> Self {
        Error::new(Kind::Conntrack, description, cause, connect_info)
    }

    pub fn new_path(cause: Option<Cause>, description: Option<String>) -> Self {
        Error::new(Kind::Path, description, cause, None)
    }

    pub fn new_token(description: &str)-> Self {
        Error::new(Kind::Token, Some(description.to_owned()), None, None)
    }

    pub fn new_timeout(description: &str)-> Self {
        Error::new(Kind::Timeout, Some(description.to_owned()), None, None)
    }

    pub fn new_transaction(description: &str)-> Self {
        Error::new(Kind::Transatcion, Some(description.to_owned()), None, None)
    }

    pub fn new_block(description: &str)-> Self {
        Error::new(Kind::Block, Some(description.to_owned()), None, None)
    }

    pub fn new_general(description: &str) -> Self {
        Error::new(Kind::General, Some(description.to_owned()), None, None)
    }

    pub fn new_id(cause: Option<Cause>, description: Option<String>) -> Self {
        Error::new(Kind::Id, description, cause, None)
    }

    pub fn new_file(cause: Option<Cause>, description: Option<String>) -> Self {
        Error::new(Kind::File, description, cause, None)
    }

    pub fn new_net(cause: Option<Cause>, description: Option<String>, connect_info: Option<SocketAddr>) -> Self {
        Error::new(Kind::Net, description, cause, connect_info)
    }

    pub fn new_io(cause: Option<Cause>, connect_info: Option<SocketAddr>) -> Self {
        Error::new(Kind::IO, None, cause, connect_info)
    }

    pub fn new_bind(cause: Option<Cause>) -> Self {
        Error::new(Kind::Bind, None, cause, None)
    }

    pub fn new_frame(cause: Option<Cause>, description: Option<String>) -> Self {
        Error::new(Kind::Frame, description, cause, None)
    }

    pub fn new_decode(description: &str) -> Self {
        Error::new(Kind::Decode, Some(description.to_owned()), None, None)
    }


    pub fn is_timeout(&self) -> bool {
        matches!(self.inner.kind, Kind::Timeout)
    }

    pub fn get_kind(&self) -> Kind {
        self.inner.kind
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
        let mut f = f.debug_tuple("yiilian_core::Error");
        f.field(&self.inner.kind);
        if let Some(ref cause) = self.inner.cause {
            f.field(cause);
        }
        if let Some(ref connect_info) = self.inner.connect_info {
            f.field(connect_info);
        }
        if let Some(ref description) = self.description {
            f.field(description);
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

pub fn hook_panic() {
    std::panic::set_hook(Box::new(|_| {
        let trace = Backtrace::new();
        BACKTRACE.with(move |b| b.borrow_mut().replace(trace));
    }));
}

pub fn trace_panic(error: &Box<dyn Any + Send>) -> (Backtrace, &str) {
    let b = BACKTRACE.with(|b| b.borrow_mut().take()).unwrap_or_default();
    let err_msg = panic_message::panic_message(error);

    (b, err_msg)
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
