use serde::{Deserialize, Serialize};

use super::rate_limiter::RateLimiter;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    pub path_on_host: String,

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
    pub socket: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CacheType {
    #[serde(rename = "Unsafe")]
    Unsafe,
    #[serde(rename = "WriteBack")]
    WriteBack,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IoEngine {
    #[serde(rename = "Sync")]
    Sync,
    #[serde(rename = "Async")]
    Async,
}
