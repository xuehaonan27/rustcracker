use serde::{Serialize, Deserialize};

use crate::utils::Json;

use super::rate_limiter::RateLimiter;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialDrive {
    // drive id
    // Required: true
    pub drive_id: String,

    // Host level path for the guest drive
    pub path_on_host: Option<String>,

    // rate limiter
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