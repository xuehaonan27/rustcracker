use crate::agent::agent::AgentError;
use std::error::Error as StdError;
use std::io;
/// Errors in rustcracker
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Fail to encode structs: {0}")]
    Encode(String),
    #[error("Fail to decode payload: {0}")]
    Decode(String),
    #[error("Configure: {0}")]
    Config(String),
    #[error("Filesys I/O: {0}")]
    FilesysIO(String),
    #[error("Firecracker: {0}")]
    Firecracker(String),
    #[error("Jailer: {0}")]
    Jailer(String),
    #[error("Agent: {0}")]
    Agent(#[from] AgentError),
    #[error("Hypervisor: {0}")]
    Hypervisor(String),
    #[error("Process: {0}")]
    Machine(String),
    /// Error communicating with the database backend.
    #[error("error communicating with database: {0}")]
    Io(#[from] io::Error),
    /// Unexpected or invalid data encountered while communicating with the firecracker.
    #[error("encountered unexpected or invalid data: {0}")]
    Protocol(String),
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub type BoxDynError = Box<dyn StdError + 'static + Send + Sync>;

#[derive(thiserror::Error, Debug)]
#[error("unexpected null; try decoding as an `Option`")]
pub struct UnexpectedNullError;

/// Format an error message as a `Protocol` error
#[macro_export]
macro_rules! err_protocol {
    ($($fmt_args:tt)*) => {
        $crate::error::Error::Protocol(
            format!(
                "{} ({}:{})",
                // Note: the format string needs to be unmodified (e.g. by `concat!()`)
                // for implicit formatting arguments to work
                format_args!($($fmt_args)*),
                module_path!(),
                line!(),
            )
        )
    };
}
