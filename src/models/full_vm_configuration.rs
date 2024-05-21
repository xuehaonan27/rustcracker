use serde::{Deserialize, Serialize};

use super::{
    balloon::Balloon, boot_source::BootSource, drive::Drive, logger::Logger,
    machine_configuration::MachineConfiguration, metrics::Metrics, mmds_config::MmdsConfig,
    network_interface::NetworkInterface, vsock::Vsock,
};

/*
FullVmConfiguration:
    type: object
    properties:
        balloon:
            $ref: "#/definitions/Balloon"
        drives:
            type: array
            description: Configurations for all block devices.
            items:
                $ref: "#/definitions/Drive"
        boot-source:
            $ref: "#/definitions/BootSource"
        logger:
            $ref: "#/definitions/Logger"
        machine-config:
            $ref: "#/definitions/MachineConfiguration"
        metrics:
            $ref: "#/definitions/Metrics"
        mmds-config:
            $ref: "#/definitions/MmdsConfig"
        network-interfaces:
            type: array
            description: Configurations for all net devices.
            items:
                $ref: "#/definitions/NetworkInterface"
        vsock:
            $ref: "#/definitions/Vsock"
*/

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FullVmConfiguration {
    #[serde(rename = "balloon", skip_serializing_if = "Option::is_none")]
    pub balloon: Option<Balloon>,

    /// Configurations for all block devices.
    #[serde(rename = "drive", skip_serializing_if = "Option::is_none")]
    pub drives: Option<Vec<Drive>>,

    #[serde(rename = "boot-source", skip_serializing_if = "Option::is_none")]
    pub boot_source: Option<BootSource>,

    #[serde(rename = "logger", skip_serializing_if = "Option::is_none")]
    pub logger: Option<Logger>,

    #[serde(rename = "machine-config", skip_serializing_if = "Option::is_none")]
    pub machine_config: Option<MachineConfiguration>,

    #[serde(rename = "metrics", skip_serializing_if = "Option::is_none")]
    pub metrics: Option<Metrics>,

    #[serde(rename = "mmds-config", skip_serializing_if = "Option::is_none")]
    pub mmds_config: Option<MmdsConfig>,

    /// Configurations for all net devices.
    #[serde(rename = "network-interfaces", skip_serializing_if = "Option::is_none")]
    pub network_interfaces: Option<Vec<NetworkInterface>>,

    #[serde(rename = "vsock", skip_serializing_if = "Option::is_none")]
    pub vsock: Option<Vsock>,
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
    pub fn with_balloon(mut self, balloon: &Balloon) -> Self {
        self.balloon = Some(balloon.to_owned());
        self
    }

    pub fn add_drive(mut self, drive: &Drive) -> Self {
        if self.drives.is_none() {
            self.drives = Some(Vec::new())
        }
        self.drives.as_mut().unwrap().push(drive.to_owned());
        self
    }

    pub fn with_boot_source(mut self, boot_source: &BootSource) -> Self {
        self.boot_source = Some(boot_source.to_owned());
        self
    }

    pub fn with_logger(mut self, logger: &Logger) -> Self {
        self.logger = Some(logger.to_owned());
        self
    }

    pub fn with_machine_config(mut self, machine_config: &MachineConfiguration) -> Self {
        self.machine_config = Some(machine_config.to_owned());
        self
    }

    pub fn with_metrics(mut self, metrics: &Metrics) -> Self {
        self.metrics = Some(metrics.to_owned());
        self
    }

    pub fn with_mmds_config(mut self, mmds_config: &MmdsConfig) -> Self {
        self.mmds_config = Some(mmds_config.to_owned());
        self
    }

    pub fn add_network_interface(mut self, network_interface: &NetworkInterface) -> Self {
        if self.network_interfaces.is_none() {
            self.network_interfaces = Some(Vec::new());
        }
        self.network_interfaces
            .as_mut()
            .unwrap()
            .push(network_interface.to_owned());
        self
    }

    pub fn with_vsock(mut self, vsock: &Vsock) -> Self {
        self.vsock = Some(vsock.to_owned());
        self
    }
}
