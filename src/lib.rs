// pub mod config;
// pub mod events;
// pub mod firecracker;
// pub mod jailer;
// pub mod local;
// pub mod machine;
// pub mod micro_http;
pub mod agent;
// pub mod database;
// pub mod machine_dev;
pub mod models;
// pub mod ops_res;
pub mod reqres;
// pub mod ser;

#[derive(Debug, thiserror::Error)]
pub enum RtckError {
    #[error("Fail to decode payload: {0}")]
    Decode(String),
}

pub type RtckResult<T> = std::result::Result<T, RtckError>;