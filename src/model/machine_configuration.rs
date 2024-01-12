use serde::{Deserialize, Serialize};

use crate::utils::Json;

use super::cpu_template::CPUTemplate;


/// # Example
/// 
/// ```
/// // This piece of code will give you a machine configuration with
/// // CPU template set to C3, 8 virtual CPU, memory
/// // size set to 1024 MiB, hyperthreading enabled and
/// // dirty pages tracking disabled.
/// use Rustcracker::model::machine_configuration::MachineConfiguration;
/// let machine_config =
///     MachineConfiguration::default()
///     .with_cpu_template("C3")
///     .with_vcpu_count(8)
///     .with_mem_size_mib(1024)
///     .set_hyperthreading(true)
///     .set_track_dirty_pages(false);
/// ```
#[derive(Serialize, Deserialize, Clone)]
pub struct MachineConfiguration {
    // cpu template
    cpu_template: Option<CPUTemplate>,

    // Flag for enabling/disabling Hyperthreading
    // Required: true
    #[serde(rename = "smt")]
    ht_enabled: bool,

    // Memory size of VM
    // Required: true
    mem_size_mib: isize,

    // Enable dirty page tracking.
    // If this is enabled, then incremental guest memory snapshots can be created.
    // These belong to diff snapshots, which contain, besides the microVM state, only the memory dirtied since
    // a previous snapshot. Full snapshots each contain a full copy of the guest memory.
    track_dirty_pages: Option<bool>,

    // Number of vCPUs (either 1 or an even number)
    // Required: true
    // Maximum: 32
    // Minimum: 1
    vcpu_count: isize,
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
            ht_enabled: false,
            mem_size_mib: 0,
            track_dirty_pages: None,
            vcpu_count: 0,
        }
    }
}

impl MachineConfiguration {
    pub fn with_cpu_template(mut self, cpu_template: impl Into<CPUTemplate>) -> Self {
        self.cpu_template = Some(cpu_template.into());
        self
    }

    pub fn set_hyperthreading(mut self, b: bool) -> Self {
        self.ht_enabled = b;
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

    pub fn demo() -> Self {
        Self {
            cpu_template: None,
            vcpu_count: 2,
            mem_size_mib: 1024,
            track_dirty_pages: Some(false),
            ht_enabled: false,
        }
    }
}
