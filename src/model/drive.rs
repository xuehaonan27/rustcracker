use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::utils::Json;

use super::rate_limiter::RateLimiter;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Drive {
    // drive id
    // Required: true
    drive_id: String,

    // Host level path for the guest drive
    // Required: true
    pub(crate) path_on_host: PathBuf,

    // is root device
    // Required: true
    is_root_device: bool,

    // is read only
    // Required: true
    is_read_only: bool,

    // Represents the unique id of the boot partition of this device.
    // It is optional and it will be taken into account
    // only if the is_root_device field is true.
    #[serde(rename = "partuuid")]
    part_uuid: Option<String>,

    // rate limiter
    rate_limiter: Option<RateLimiter>,
    // // cache type
    // cache_type:

    // // io engine
    // io_engine:

    // // socket
    // socket: Option<PathBuf>
}

impl<'a> Json<'a> for Drive {
    type Item = Drive;
}

impl Drive {
    pub fn demo() -> Self {
        Self {
            drive_id: "rootfs".into(),
            path_on_host: "bionic.rootfs.ext4".into(),
            is_root_device: true,
            is_read_only: false,
            part_uuid: None,
            rate_limiter: None,
        }
    }

    pub fn new() -> Self {
        Self {
            drive_id: "".into(),
            path_on_host: "".into(),
            is_root_device: false,
            is_read_only: false,
            part_uuid: None,
            rate_limiter: None,
        }
    }

    pub fn with_drive_id<S>(mut self, id: S) -> Self
    where
        S: Into<String>
    {
        self.drive_id = id.into();
        self
    }

    pub fn with_part_uuid(mut self, uuid: String) -> Self {
        self.part_uuid = Some(uuid);
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

    pub fn with_drive_path<S>(mut self, path: S) -> Self
    where
        S: Into<PathBuf>
    {
        self.path_on_host = path.into();
        self
    }
    
    pub fn with_rate_limiter(mut self, limiter: RateLimiter) -> Self {
        self.rate_limiter = Some(limiter);
        self
    }

    pub fn get_drive_id(&self) -> String {
        self.drive_id.to_owned()
    }
}
