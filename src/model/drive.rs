use std::path::PathBuf;

use log::error;
use serde::{Deserialize, Serialize};

use crate::{client::machine::MachineError, utils::Json};

use super::rate_limiter::RateLimiter;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Drive {
    /// drive id
    /// Required: true
    #[serde(rename = "drive_id")]
    pub drive_id: String,

    /// partuuid
    /// Represents the unique id of the boot partition of this device.
    /// It is optional and it will be taken into account
    /// only if the is_root_device field is true.
    #[serde(rename = "partuuid", skip_serializing_if = "Option::is_none")]
    pub partuuid: Option<String>,

    /// is root device
    /// Required: true
    #[serde(rename = "is_root_device")]
    pub is_root_device: bool,

    /// cache type
    /// Represents the caching strategy for the block device.
    #[serde(rename = "cache_type", skip_serializing_if = "Option::is_none")]
    pub cache_type: Option<CacheType>,

    /// VirtioBlock specific parameters:
    /// Is block read only.
    /// This field is required for virtio-block config and should be omitted for vhost-user-block configuration.
    /// Required: true
    #[serde(rename = "is_read_only")]
    pub is_read_only: bool,

    /// VirtioBlock specific parameters:
    /// Host level path for the guest drive.
    /// This field is required for virtio-block config and should be omitted for vhost-user-block configuration.
    /// Required: true
    #[serde(rename = "path_on_host")]
    pub path_on_host: PathBuf,

    /// VirtioBlock specific parameters:
    /// rate limiter
    #[serde(rename = "rate_limiter", skip_serializing_if = "Option::is_none")]
    pub rate_limiter: Option<RateLimiter>,

    /// VirtioBlock specific parameters:
    /// Type of the IO engine used by the device. "Async" is supported on
    /// host kernels newer than 5.10.51.
    /// This field is optional for virtio-block config and should be omitted for vhost-user-block configuration.
    #[serde(rename = "io_engine", skip_serializing_if = "Option::is_none")]
    pub io_engine: Option<IoEngine>,

    /// VhostUserBlock specific parameters
    /// Path to the socket of vhost-user-block backend.
    /// This field is required for vhost-user-block config should be omitted for virtio-block configuration.
    #[serde(rename = "socket", skip_serializing_if = "Option::is_none")]
    pub socket: Option<PathBuf>,
}

impl<'a> Json<'a> for Drive {
    type Item = Drive;
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum CacheType {
    #[serde(rename = "Unsafe")]
    Unsafe,
    #[serde(rename = "WriteBack")]
    WriteBack,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum IoEngine {
    #[serde(rename = "Sync")]
    Sync,
    #[serde(rename = "Async")]
    Async,
}

impl Drive {
    pub fn new() -> Self {
        Self {
            drive_id: "".into(),
            path_on_host: "".into(),
            is_root_device: false,
            is_read_only: false,
            partuuid: None,
            rate_limiter: None,
            cache_type: None,
            io_engine: None,
            socket: None,
        }
    }

    pub fn with_drive_id<S>(mut self, id: S) -> Self
    where
        S: Into<String>,
    {
        self.drive_id = id.into();
        self
    }

    pub fn with_part_uuid(mut self, uuid: String) -> Self {
        self.partuuid = Some(uuid);
        self
    }

    pub fn set_root_device(mut self, is_root_device: bool) -> Self {
        self.is_root_device = is_root_device;
        self
    }

    pub fn set_read_only(mut self, read_only: bool) -> Self {
        self.is_read_only = read_only;
        self
    }

    pub fn with_drive_path<S>(mut self, path: impl Into<PathBuf>) -> Self {
        self.path_on_host = path.into();
        self
    }

    pub fn set_drive_path(&mut self, path: impl Into<PathBuf>) {
        self.path_on_host = path.into();
    }

    pub fn with_rate_limiter(mut self, limiter: RateLimiter) -> Self {
        self.rate_limiter = Some(limiter);
        self
    }

    pub fn get_drive_id(&self) -> String {
        self.drive_id.to_owned()
    }

    pub fn is_root_device(&self) -> bool {
        self.is_root_device
    }

    pub fn get_path_on_host(&self) -> PathBuf {
        self.path_on_host.to_owned()
    }

    #[must_use="must validate Drive before putting it to microVm"]
    pub fn validate(&self) -> Result<(), MachineError> {
        if self.drive_id == "".to_string() {
            error!(target: "Drive::validate", "cannot assign empty id to the drive");
            return Err(MachineError::Validation(
                "cannot assign empty id to the drive".to_string(),
            ));
        }

        if self.partuuid.is_some() && self.partuuid.as_ref().unwrap() == &"".to_string() {
            error!(target: "Drive::validate", "cannot assign empty uuid to the drive, leave it None");
            return Err(MachineError::Validation(
                "cannot assign empty uuid to the drive, leave it None".to_string(),
            ));
        }

        if let Err(e) = std::fs::metadata(&self.path_on_host) {
            error!(target: "Drive::validate", "fail to stat drive path {}: {}", self.path_on_host.display(), e.to_string());
            return Err(MachineError::Validation(format!(
                "fail to stat drive path {}: {}",
                self.path_on_host.display(),
                e.to_string()
            )));
        }

        if self.socket.is_some() && std::fs::metadata(self.socket.as_ref().unwrap()).is_err() {
            error!(target: "Drive::validate", "fail to stat drive socket. This field is required for vhost-user-block config should be omitted for virtio-block configuration.");
            return Err(MachineError::Validation(format!("fail to stat drive socket {}. This field is required for vhost-user-block config should be omitted for virtio-block configuration.", self.path_on_host.display())));
        }

        Ok(())
    }
}
