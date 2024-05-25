use serde::{Deserialize, Serialize};

use super::*;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FullVmConfiguration {
    #[serde(rename = "balloon", skip_serializing_if = "Option::is_none")]
    pub balloon: Option<balloon::Balloon>,

    /// Configurations for all block devices.
    #[serde(rename = "drive", skip_serializing_if = "Option::is_none")]
    pub drives: Option<Vec<drive::Drive>>,

    #[serde(rename = "boot-source", skip_serializing_if = "Option::is_none")]
    pub boot_source: Option<boot_source::BootSource>,

    #[serde(rename = "logger", skip_serializing_if = "Option::is_none")]
    pub logger: Option<logger::Logger>,

    #[serde(rename = "machine-config", skip_serializing_if = "Option::is_none")]
    pub machine_config: Option<machine_configuration::MachineConfiguration>,

    #[serde(rename = "metrics", skip_serializing_if = "Option::is_none")]
    pub metrics: Option<metrics::Metrics>,

    #[serde(rename = "mmds-config", skip_serializing_if = "Option::is_none")]
    pub mmds_config: Option<mmds_config::MmdsConfig>,

    /// Configurations for all net devices.
    #[serde(rename = "network-interfaces", skip_serializing_if = "Option::is_none")]
    pub network_interfaces: Option<Vec<network_interface::NetworkInterface>>,

    #[serde(rename = "vsock", skip_serializing_if = "Option::is_none")]
    pub vsock: Option<vsock::Vsock>,
}

impl Default for FullVmConfiguration {
    fn default() -> Self {
        Self {
            balloon: None,
            drives: None,
            boot_source: None,
            logger: None,
            machine_config: None,
            metrics: None,
            mmds_config: None,
            network_interfaces: None,
            vsock: None,
        }
    }
}

impl FullVmConfiguration {
    pub fn with_balloon(mut self, balloon: &balloon::Balloon) -> Self {
        self.balloon = Some(balloon.to_owned());
        self
    }

    pub fn add_drive(mut self, drive: &drive::Drive) -> Self {
        if self.drives.is_none() {
            self.drives = Some(Vec::new())
        }
        self.drives.as_mut().unwrap().push(drive.to_owned());
        self
    }

    pub fn with_boot_source(mut self, boot_source: &boot_source::BootSource) -> Self {
        self.boot_source = Some(boot_source.to_owned());
        self
    }

    pub fn with_logger(mut self, logger: &logger::Logger) -> Self {
        self.logger = Some(logger.to_owned());
        self
    }

    pub fn with_machine_config(
        mut self,
        machine_config: &machine_configuration::MachineConfiguration,
    ) -> Self {
        self.machine_config = Some(machine_config.to_owned());
        self
    }

    pub fn with_metrics(mut self, metrics: &metrics::Metrics) -> Self {
        self.metrics = Some(metrics.to_owned());
        self
    }

    pub fn with_mmds_config(mut self, mmds_config: &mmds_config::MmdsConfig) -> Self {
        self.mmds_config = Some(mmds_config.to_owned());
        self
    }

    pub fn add_network_interface(
        mut self,
        network_interface: &network_interface::NetworkInterface,
    ) -> Self {
        if self.network_interfaces.is_none() {
            self.network_interfaces = Some(Vec::new());
        }
        self.network_interfaces
            .as_mut()
            .unwrap()
            .push(network_interface.to_owned());
        self
    }

    pub fn with_vsock(mut self, vsock: &vsock::Vsock) -> Self {
        self.vsock = Some(vsock.to_owned());
        self
    }
}
