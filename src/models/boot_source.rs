use serde::{Deserialize, Serialize};

/// Boot source descriptor.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BootSource {
    /// Kernel boot arguments
    pub boot_args: Option<String>,

    /// Host level path to the initrd image used to boot the guest
    pub initrd_path: Option<String>,

    /// Host level path to the kernel image used to boot the guest
    /// Required: true
    pub kernel_image_path: String,
}
