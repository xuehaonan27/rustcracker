use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::utils::Json;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BootSource {
    // Kernel boot arguments
    pub boot_args: Option<String>,

    // Host level path to the initrd image used to boot the guest
    pub initrd_path: Option<PathBuf>,

    // Host level path to the kernel image used to boot the guest
    // Required: true
    pub kernel_image_path: PathBuf,
}

impl<'a> Json<'a> for BootSource {
    type Item = BootSource;
}

impl Default for BootSource {
    fn default() -> Self {
        Self {
            boot_args: None,
            initrd_path: None,
            kernel_image_path: "".into(),
        }
    }
}

impl BootSource {
    pub fn from_kernel_image_path<S>(kernel_image_path: S) -> Self
    where
        S: Into<PathBuf>,
    {
        Self {
            boot_args: None,
            initrd_path: None,
            kernel_image_path: kernel_image_path.into(),
        }
    }

    pub fn with_kernel_image_path<S>(mut self, kernel_image_path: S) -> Self
    where
        S: Into<PathBuf>
    {
        self.kernel_image_path = kernel_image_path.into();
        self
    }

    pub fn with_boot_args<S>(mut self, boot_args: S) -> Self
    where
        S: Into<String>,
    {
        self.boot_args = Some(boot_args.into());
        self
    }

    pub fn with_initrd_path<S>(mut self, path: S) -> Self
    where
        S: Into<PathBuf>,
    {
        self.initrd_path = Some(path.into());
        self
    }
}