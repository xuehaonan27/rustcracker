use agent::agent::AgentError;
use log::*;
pub mod agent;
pub mod config;
pub mod firecracker;
pub mod hypervisor;
pub mod jailer;
pub mod models;
pub mod options;
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
}

pub type RtckResult<T> = std::result::Result<T, RtckError>;

#[doc(hidden)]
pub(crate) fn handle_entry<T: Clone>(option: &Option<T>, name: &'static str) -> RtckResult<T> {
    option.clone().ok_or_else(|| {
        let msg = format!("Missing {name} entry");
        error!("{msg}");
        RtckError::Config(msg)
    })
}

#[doc(hidden)]
fn handle_entry_default<T: Clone>(entry: &Option<T>, default: T) -> T {
    if entry.as_ref().is_some() {
        entry.as_ref().unwrap().clone()
    } else {
        default
    }
}

#[doc(hidden)]
fn handle_entry_ref<'a, T>(entry: &'a Option<T>, name: &'static str) -> RtckResult<&'a T> {
    entry.as_ref().ok_or_else(|| {
        let msg = format!("Missing {name} entry");
        error!("{msg}");
        RtckError::Jailer(msg)
    })
}
