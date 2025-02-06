use log::*;
mod agent;
pub mod config;
mod firecracker;
pub mod hypervisor;
mod jailer;
pub mod models;
pub mod options;
mod raii;
mod reqres;
pub use crate::hypervisor::Hypervisor;
pub use crate::hypervisor::HypervisorSync;

/// Errors in rustcracker
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
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Agent: {0}")]
    Agent(String),
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
