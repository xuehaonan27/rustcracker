use log::error;
use serde::{Deserialize, Serialize};

use crate::{components::machine::MachineError, utils::Json};

use super::cpu_template::CPUTemplate;

/// # Example
///
/// ```
/// // This piece of code will give you a machine configuration with
/// // CPU template set to C3, 8 virtual CPU, memory
/// // size set to 1024 MiB, hyperthreading enabled and
/// // dirty pages tracking disabled.
/// use rustcracker::model::machine_configuration::MachineConfiguration;
/// use rustcracker::model::cpu_template::{CPUTemplate, CPUTemplateString};
/// let machine_config =
///     MachineConfiguration::default()
///         .with_cpu_template(&CPUTemplate(CPUTemplateString::C3))
///         .with_vcpu_count(8)
///         .with_mem_size_mib(1024)
///         .set_hyperthreading(true)
///         .set_track_dirty_pages(false);
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MachineConfiguration {
    /// cpu template
    #[serde(rename = "cpu_template", skip_serializing_if = "Option::is_none")]
    pub cpu_template: Option<CPUTemplate>,

    /// Flag for enabling/disabling Hyperthreading
    /// Required: true
    #[serde(rename = "smt", skip_serializing_if = "Option::is_none")]
    pub ht_enabled: Option<bool>,

    /// Memory size of VM
    /// Required: true
    #[serde(rename = "mem_size_mib")]
    pub mem_size_mib: isize,

    /// Enable dirty page tracking.
    /// If this is enabled, then incremental guest memory snapshots can be created.
    /// These belong to diff snapshots, which contain, besides the microVM state, only the memory dirtied since
    /// a previous snapshot. Full snapshots each contain a full copy of the guest memory.
    #[serde(rename = "track_dirty_pages", skip_serializing_if = "Option::is_none")]
    pub track_dirty_pages: Option<bool>,

    /// Number of vCPUs (either 1 or an even number)
    /// Required: true
    /// Maximum: 32
    /// Minimum: 1
    #[serde(rename = "vcpu_count")]
    pub vcpu_count: isize,
}

impl<'a> Json<'a> for MachineConfiguration {
    type Item = MachineConfiguration;
}

impl Default for MachineConfiguration {
    /// Get a default MachineConfiguration instance.
    /// By default, it disables hyperthreading, set memory
    /// size to 0 and allocate no vCPU for the machine.
    fn default() -> Self {
        Self {
            cpu_template: None,
            ht_enabled: Some(false),
            mem_size_mib: 0,
            track_dirty_pages: None,
            vcpu_count: 0,
        }
    }
}

impl MachineConfiguration {
    pub fn with_cpu_template(mut self, cpu_template: &CPUTemplate) -> Self {
        self.cpu_template = Some(cpu_template.to_owned());
        self
    }

    pub fn set_hyperthreading(mut self, b: bool) -> Self {
        self.ht_enabled = Some(b);
        self
    }

    pub fn with_mem_size_mib(mut self, m: isize) -> Self {
        self.mem_size_mib = m;
        self
    }

    pub fn set_track_dirty_pages(mut self, b: bool) -> Self {
        self.track_dirty_pages = Some(b);
        self
    }

    pub fn with_vcpu_count(mut self, c: isize) -> Self {
        self.vcpu_count = c;
        self
    }

    pub fn get_vcpu_count(&self) -> isize {
        self.vcpu_count
    }

    pub fn is_ht_enabled(&self) -> bool {
        if self.ht_enabled.is_none() {
            return false;
        } else {
            return *self.ht_enabled.as_ref().unwrap();
        }
    }

    pub fn get_mem_size_in_mib(&self) -> isize {
        self.mem_size_mib
    }

    #[must_use="must validate MachineConfiguration before putting it to microVm"]
    pub fn validate(&self) -> Result<(), MachineError> {
        if self.vcpu_count < 1 {
            error!(target: "MachineConfiguration::validate", "machine needs a non-zero vcpu count");
            return Err(MachineError::Validation(
                "machine needs a non-zero vcpu count".to_string(),
            ));
        }
        if self.mem_size_mib < 1 {
            error!(target: "MachineConfiguration::validate", "machine needs a non-zero amount of memory");
            return Err(MachineError::Validation(
                "machine needs a non-zero amount of memory".to_string(),
            ));
        }

        Ok(())
    }
}
