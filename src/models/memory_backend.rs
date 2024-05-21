use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MemoryBackend {
    #[serde(rename = "backend_type")]
    pub backend_type: BackendType,
    /// Based on 'backend_type' it is either
    /// 1) Path to the file that contains the guest memory to be loaded
    /// 2) Path to the UDS where a process is listening for a UFFD initialization
    /// control payload and open file descriptor that it can use to serve this
    /// process's guest memory page faults
    #[serde(rename = "backend_path")]
    pub backend_path: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum BackendType {
    #[serde(rename = "File")]
    File,
    #[serde(rename = "Uffd")]
    Uffd,
}
