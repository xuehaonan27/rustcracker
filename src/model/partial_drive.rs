use std::path::PathBuf;

use log::error;
use serde::{Deserialize, Serialize};

use crate::{client::machine::MachineError, utils::Json};

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
    pub path_on_host: Option<PathBuf>,

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

    pub fn validate(&self) -> Result<(), MachineError> {
        if self.path_on_host.is_some()
            && std::fs::metadata(self.path_on_host.as_ref().unwrap()).is_err()
        {
            error!(target: "PartialDrive::validate", "fail to stat drive {}", self.path_on_host.as_ref().unwrap().display());
            return Err(MachineError::Validation(format!(
                "fail to stat drive {}",
                self.path_on_host.as_ref().unwrap().display()
            )));
        }

        Ok(())
    }
}
