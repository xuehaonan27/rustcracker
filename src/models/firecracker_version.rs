use serde::{Deserialize, Serialize};

/// Describes the Firecracker version.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FirecrackerVersion {
    /// Firecracker build version.
    #[serde(rename = "firecracker_version")]
    pub firecracker_version: String,
}

impl Default for FirecrackerVersion {
    fn default() -> Self {
        Self {
            firecracker_version: "".into(),
        }
    }
}
