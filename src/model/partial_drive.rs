use serde::{Deserialize, Serialize};

use crate::utils::Json;

use super::rate_limiter::RateLimiter;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialDrive {
    /// drive id
    /// Required: true
    #[serde(rename = "drive_id")]
    pub drive_id: String,

    /// Host level path for the guest drive
    /// This field is optional for virtio-block config
    /// and should be omitted for vhost-user-block configuration.
    #[serde(rename = "path_on_host", skip_serializing_if = "Option::is_none")]
    pub path_on_host: Option<String>,

    /// rate limiter
    #[serde(rename = "rate_limiter", skip_serializing_if = "Option::is_none")]
    pub rate_limiter: Option<RateLimiter>,
}

impl<'a> Json<'a> for PartialDrive {
    type Item = PartialDrive;
}

impl PartialDrive {
    pub fn get_drive_id(&self) -> String {
        self.drive_id.to_owned()
    }
}
