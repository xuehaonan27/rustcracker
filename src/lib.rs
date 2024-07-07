use agent::agent::AgentError;

pub mod config;
// pub mod events;
pub mod firecracker;
pub mod jailer;
pub mod local;
// pub mod machine;
// pub mod micro_http;
pub mod agent;
// pub mod database;
pub mod machine_dev;
pub mod models;
// pub mod ops_res;
pub mod reqres;
// pub mod ser;

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
}

pub type RtckResult<T> = std::result::Result<T, RtckError>;
