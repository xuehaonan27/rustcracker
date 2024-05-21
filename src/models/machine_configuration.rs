use serde::{Deserialize, Serialize};

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
