use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::utils::Json;

use super::rate_limiter::RateLimiter;
/// Defines a network interface.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkInterface {
    /// If this field is set, the device model will reply to
    /// HTTP GET requests sent to the MMDS address via this interface.
    /// In this case, both ARP requests for 169.254.169.254 and TCP
    /// segments heading to the same address are intercepted by the
    /// device model, and do not reach the associated TAP device.
    // pub allow_mmds_requests: Option<bool>,

    /// guest mac
    #[serde(rename = "guest_mac", skip_serializing_if = "Option::is_none")]
    pub guest_mac: Option<String>,

    /// Host level path for the guest network interface
    /// Required: true
    #[serde(rename = "host_dev_name")]
    pub host_dev_name: PathBuf,

    /// iface id
    /// Required: true
    #[serde(rename = "iface_id")]
    pub iface_id: String,

    /// rx rate limiter
    #[serde(rename = "rx_rate_limiter", skip_serializing_if = "Option::is_none")]
    pub rx_rate_limiter: Option<RateLimiter>,

    /// tx rate limiter
    #[serde(rename = "tx_rate_limiter", skip_serializing_if = "Option::is_none")]
    pub tx_rate_limiter: Option<RateLimiter>,
}

impl<'a> Json<'a> for NetworkInterface {
    type Item = NetworkInterface;
}

impl NetworkInterface {
    pub fn get_iface_id(&self) -> String {
        self.iface_id.to_owned()
    }
}

impl Default for NetworkInterface {
    fn default() -> Self {
        Self {
            // allow_mmds_requests: None,
            guest_mac: None,
            host_dev_name: "".into(),
            iface_id: "".into(),
            rx_rate_limiter: None,
            tx_rate_limiter: None,
        }
    }
}

impl NetworkInterface {
    // pub fn validate(&self) -> Result<(), MachineError> {
    //     if let Err(e) = std::fs::metadata(&self.host_dev_name) {
    //         error!(target: "NetworkInterface::validate", "fail to stat network interface {}", e);
    //         return Err(MachineError::Validation(format!(
    //             "fail to stat network interface {}", e.to_string()
    //         )))
    //     }

    //     Ok(())
    // }
    // pub fn set_allow_mmds_requests(mut self, b: bool) -> Self {
    //     self.allow_mmds_requests = Some(b);
    //     self
    // }

    pub fn with_guest_mac(mut self, mac: impl Into<String>) -> Self {
        self.guest_mac = Some(mac.into());
        self
    }

    pub fn with_host_dev_name(mut self, path: impl Into<PathBuf>) -> Self {
        self.host_dev_name = path.into();
        self
    }

    pub fn with_iface_id(mut self, iface_id: impl Into<String>) -> Self {
        self.iface_id = iface_id.into();
        self
    }

    pub fn with_rx_rate_limiter(mut self, limiter: RateLimiter) -> Self {
        self.rx_rate_limiter = Some(limiter);
        self
    }

    pub fn with_tx_rate_limiter(mut self, limiter: RateLimiter) -> Self {
        self.tx_rate_limiter = Some(limiter);
        self
    }    
}
