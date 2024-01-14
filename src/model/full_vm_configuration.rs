use serde::{Deserialize, Serialize};

use crate::utils::Json;

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

#[derive(Serialize, Deserialize)]
pub struct FullVmConfiguration {
    balloon: Balloon,

    drives: Vec<Drive>,

    #[serde(rename = "boot-source")]
    boot_source: BootSource,

    logger: Logger,

    #[serde(rename = "machine-config")]
    machine_config: MachineConfiguration,

    metrics: Metrics,

    #[serde(rename = "mmds-config")]
    mmds_config: MmdsConfig,

    #[serde(rename = "network-interfaces")]
    network_interfaces: Vec<NetworkInterface>,

    vsock: Vsock,
}

impl<'a> Json<'a> for FullVmConfiguration {
    type Item = FullVmConfiguration;
}

impl Default for FullVmConfiguration {
    fn default() -> Self {
        Self {
            balloon: Balloon::new(),
            drives: Vec::new(),
            boot_source: BootSource::default(),
            logger: Logger::default(),
            machine_config: MachineConfiguration::default(),
            metrics: Metrics::default(),
            mmds_config: MmdsConfig::default(),
            network_interfaces: Vec::new(),
            vsock: Vsock::default(),
        }
    }
}

impl FullVmConfiguration {
    pub fn with_balloon(mut self, balloon: Balloon) -> Self {
        self.balloon = balloon;
        self
    }

    pub fn add_drive(mut self, drive: Drive) -> Self {
        self.drives.push(drive);
        self
    }

    pub fn with_boot_source(mut self, boot_source: BootSource) -> Self {
        self.boot_source = boot_source;
        self
    }

    pub fn with_logger(mut self, logger: Logger) -> Self {
        self.logger = logger;
        self
    }

    pub fn with_machine_config(mut self, machine_config: MachineConfiguration) -> Self {
        self.machine_config = machine_config;
        self
    }

    pub fn with_metrics(mut self, metrics: Metrics) -> Self {
        self.metrics = metrics;
        self
    }

    pub fn with_mmds_config(mut self, mmds_config: MmdsConfig) -> Self {
        self.mmds_config = mmds_config;
        self
    }

    pub fn add_network_interface(mut self, network_interface: NetworkInterface) -> Self {
        self.network_interfaces.push(network_interface);
        self
    }

    pub fn with_vsock(mut self, vsock: Vsock) -> Self {
        self.vsock = vsock;
        self
    }
}