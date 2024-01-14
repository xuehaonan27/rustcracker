use serde::{Deserialize, Serialize};

use crate::utils::Json;

#[derive(Serialize, Deserialize, Clone)]
pub struct FirecrackerVersion {
    firecracker_version: String,
}

impl<'a> Json<'a> for FirecrackerVersion {
    type Item = FirecrackerVersion;
}

impl Default for FirecrackerVersion {
    fn default() -> Self {
        Self {
            firecracker_version: "".into(),
        }
    }
}

impl FirecrackerVersion {
    pub fn with_version(mut self, version: String) -> Self {
        self.firecracker_version = version;
        self
    }
}