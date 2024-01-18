use serde::{Deserialize, Serialize};

use crate::utils::Json;

/// Defines the MMDS configuration.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MmdsConfig {
    /// Enumeration indicating the MMDS version to be configured.
    #[serde(rename = "version", skip_serializing_if = "Option::is_none")]
    pub version: Option<Version>,
    /// A valid IPv4 link-local address.
    /// format: "169.254.([1-9]|[1-9][0-9]|1[0-9][0-9]|2[0-4][0-9]|25[0-4]).([0-9]|[1-9][0-9]|1[0-9][0-9]|2[0-4][0-9]|25[0-5])"
    /// default: "169.254.169.254"
    #[serde(rename = "ipv4_address", skip_serializing_if = "Option::is_none")]
    pub ipv4_address: Option<String>,
    /// List of the network interface IDs capable of forwarding packets to
    /// the MMDS. Network interface IDs mentioned must be valid at the time
    /// of this request. The net device model will reply to HTTP GET requests
    /// sent to the MMDS address via the interfaces mentioned. In this
    /// case, both ARP requests and TCP segments heading to `ipv4_address`
    /// are intercepted by the device model, and do not reach the associated
    /// TAP device.
    #[serde(rename = "network_interfaces")]
    pub network_interfaces: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Version {
    #[serde(rename = "V1")]
    V1,
    #[serde(rename = "V2")]
    V2,
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
        Self {
            ipv4_address: None,
            version: None,
            network_interfaces: Vec::new(),
        }
    }
}
