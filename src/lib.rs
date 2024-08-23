use agent::agent::AgentError;
use hplog::LoggerError;

pub mod config;
pub mod firecracker;
pub mod jailer;
pub mod agent;
pub mod hypervisor;
pub mod hplog;
pub mod models;
pub mod raii;
pub mod reqres;
pub mod sync_hypervisor;

#[derive(Debug, thiserror::Error)]
pub enum RtckError {
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
    #[error("Logger: {0}")]
    Logger(#[from] LoggerError),
}

pub type RtckResult<T> = std::result::Result<T, RtckError>;

#[doc(hidden)]
pub(crate) fn handle_entry<T: Clone>(option: &Option<T>) -> RtckResult<T> {
    option
        .clone()
        .ok_or(RtckError::Config("missing config entry".to_string()))
}
