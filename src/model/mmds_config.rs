use serde::{Deserialize, Serialize};

use crate::utils::Json;
#[derive(Serialize, Deserialize)]
pub struct MmdsConfig {
    // A valid IPv4 link-local address.
    ipv4_address: Option<String>,
}

impl<'a> Json<'a> for MmdsConfig {
    type Item = MmdsConfig;
}

pub type MmdsContentsObject = String;

impl<'a> Json<'a> for MmdsContentsObject {
    type Item = MmdsContentsObject;
}

impl Default for MmdsConfig {
    fn default() -> Self {
        Self { ipv4_address: None }
    }
}
