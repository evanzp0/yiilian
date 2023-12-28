
pub type Result<T> = std::result::Result<T, YiiLianError>;

#[derive(thiserror::Error, Debug)]
pub enum YiiLianError {
    // Failure to parse bytes of a frame
    #[error("Failed to parse packet bytes: {0}")]
    FrameParse(#[source] anyhow::Error),

    #[error("Failed to serialize msg: {0}")]
    FrameSerialization(#[source] anyhow::Error),

    #[error("General error: {0}")]
    General(#[source] anyhow::Error),

    #[error("Io channel Closed error: {0}")]
    IoChannelClosed(#[source] anyhow::Error),

    #[error("Transaction channel Closed error: {0}")]
    TranChannelClosed(#[source] anyhow::Error),

    #[error("Connection tracking error: {0}")]
    Conntrack(#[source] anyhow::Error),

    #[error("Socket send error: {0}")]
    SocketSend(#[source] std::io::Error),

    #[error("Socket recv error: {0}")]
    SocketRecv(#[source] std::io::Error),

    #[error("Operation timed out: {0}")]
    Timeout(#[source] anyhow::Error),

    /// This error is a hack for signaling shutdown.
    /// Don't use unless you're sure you know what you're doing.
    #[error("It's time to shutdown tasks: {0}")]
    Shutdown(#[source] anyhow::Error),

    /// Indicates that the Message type you're trying to build requires more information.
    #[error("{0} is required")]
    BuilderMissingField(&'static str),

    /// Indicates that the builder is in an invalid/ambiguous state to build the desired
    /// Message type.
    #[error("Builder state invalid: {0}")]
    BuilderInvalidCombo(&'static str),

    #[error("Address binding  failed: {0}")]
    Bind(#[source] std::io::Error),

    /// Indicates that the query token is expired or not our's token
    #[error("Token is invalid: {0}")]
    TokenInvalid(#[source] anyhow::Error),

    /// Indicates that the transaction is handling
    #[error("Transactin is exists: {0}")]
    TransatcionExist(#[source] anyhow::Error),

    /// Indicates that the deserialize file is failure
    #[error("Deserialize file failure: {0}")]
    DeserializeFileFailed(#[source] anyhow::Error),

    /// Indicates that the deserialize file is failure
    #[error("Path operate found: {0}")]
    PathOperateFound(#[source] anyhow::Error),

    /// Indicates that the deserialize file is failure
    #[error("File operate failed: {0}")]
    FileOperateFailed(#[source] anyhow::Error),

    /// Indicates that the remote address is in the block list
    #[error("Interrupt by block: {0}")]
    InterruptByBlock(#[source] anyhow::Error)
}

impl YiiLianError {
    pub fn new_bind(source: std::io::Error) -> YiiLianError {
        YiiLianError::Bind(source)
    }

    pub fn new_recv(source: std::io::Error) -> YiiLianError {
        YiiLianError::SocketRecv(source)
    }

    pub fn new_send(source: std::io::Error) -> YiiLianError {
        YiiLianError::SocketSend(source)
    }

    pub fn new_frame_parse<E>(source: anyhow::Error) -> YiiLianError {
        YiiLianError::FrameParse(source)
    }
}
