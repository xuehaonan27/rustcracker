use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    models::{
        balloon::Balloon, drive::Drive, logger::LogLevel,
        machine_configuration::MachineConfiguration, network_interface::NetworkInterface,
        vsock::Vsock,
    },
    RtckError, RtckErrorClass, RtckResult,
};

/// Firecracker configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FirecrackerConfig {
    /// log_path defines the file path where the Firecracker log is located.
    /// will be disabled if log_fifo is set
    pub log_path: Option<String>,

    /// log_level defines the verbosity of Firecracker logging.  Valid values are
    /// "Error", "Warning", "Info", and "Debug", and are case-sensitive.
    pub log_level: Option<LogLevel>,

    /// log_clear defines whether rustcracker should remove log files after microVM
    /// was removed. Default to false.
    pub log_clear: Option<bool>,

    /// metrics_path defines the file path where the Firecracker metrics
    /// is located.
    /// will be disabled if metrics_fifo is set
    pub metrics_path: Option<String>,

    /// metrics_clear defines whether rustcracker should remove log files after microVM
    /// was removed. Default to false.
    pub metrics_clear: Option<bool>,

    /// kernel_image_path defines the file path where the kernel image is located.
    /// The kernel image must be an uncompressed ELF image.
    pub kernel_image_path: Option<String>,

    /// initrd_path defines the file path where initrd image is located.
    /// This parameter is optional.
    pub initrd_path: Option<String>,

    /// kernel_args defines the command-line arguments that should be passed to
    /// the kernel.
    pub kernel_args: Option<String>,

    /// drives specifies BlockDevices that should be made available to the
    /// microVM.
    pub drives: Option<Vec<Drive>>,

    /// network_interfaces specifies the tap devices that should be made available
    /// to the microVM.
    pub network_interfaces: Option<Vec<NetworkInterface>>,

    /// fifo_log_writer is an io.Writer(Stdio) that is used to redirect the contents of the
    /// fifo log to the writer.
    /// pub(crate) fifo_log_writer: Option<std::process::Stdio>,
    // pub fifo_log_writer: Option<String>,

    /// vsock_devices specifies the vsock devices that should be made available to
    /// the microVM.
    pub vsock_devices: Option<Vec<Vsock>>,

    /// machine_cfg represents the firecracker microVM process configuration
    pub machine_cfg: Option<MachineConfiguration>,

    /// (Optional) vmid is a unique identifier for this VM. It's set to a
    /// random uuid if not provided by the user. It's used to set Firecracker's instance ID.
    /// If CNI configuration is provided as part of NetworkInterfaces,
    /// the vmid is used to set CNI ContainerID and create a network namespace path.
    pub vmid: Option<String>,

    /// net_ns represents the path to a network namespace handle. If present, the
    /// application will use this to join the associated network namespace
    pub net_ns: Option<String>,

    /// network_clear defines whether rustcracker should clear networking
    /// after the microVM is removed. Default to false.
    pub network_clear: Option<bool>,

    /// seccomp_level specifies whether seccomp filters should be installed and how
    /// restrictive they should be. Possible values are:
    ///
    ///	0 : (default): disabled.
    ///	1 : basic filtering. This prohibits syscalls not whitelisted by Firecracker.
    ///	2 : advanced filtering. This adds further checks on some of the
    ///			parameters of the allowed syscalls.
    pub seccomp_level: Option<usize>,

    /// mmds_address is IPv4 address used by guest applications when issuing requests to MMDS.
    /// It is possible to use a valid IPv4 link-local address (169.254.0.0/16).
    /// If not provided, the default address (169.254.169.254) will be used.
    pub mmds_address: Option<std::net::Ipv4Addr>,

    /// balloon is Balloon device that is to be put to the machine
    pub balloon: Option<Balloon>,

    /// init_metadata is initial metadata that is to be assigned to the machine
    pub init_metadata: Option<String>,
}

impl FirecrackerConfig {
    pub fn validate(&self) -> RtckResult<()> {
        match &self.log_path {
            None => (),
            Some(path) => {
                let path = PathBuf::from(path);
                if path.exists() {
                    return Err(RtckError::new(
                        RtckErrorClass::ConfigError,
                        "Log path already occupied".to_string(),
                    ));
                }
            }
        }

        match &self.metrics_path {
            None => (),
            Some(path) => {
                let path = PathBuf::from(path);
                if path.exists() {
                    return Err(RtckError::new(
                        RtckErrorClass::ConfigError,
                        "Metrics path already occupied".to_string(),
                    ));
                }
            }
        }

        match &self.kernel_image_path {
            None => {
                return Err(RtckError::new(
                    RtckErrorClass::ConfigError,
                    "Kernel image file must be specified in configuration".to_string(),
                ))
            }
            Some(path) => {
                let path = PathBuf::from(path);
                if !path.exists() || !path.is_file() {
                    return Err(RtckError::new(
                        RtckErrorClass::ConfigError,
                        "Kernel image file missing".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    pub fn to_vec(&self) -> RtckResult<Vec<u8>> {
        Ok(serde_json::to_vec(&self)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JailerConfig {
    // `gid` the jailer switches to as it execs the target binary.
    pub gid: Option<u32>,

    // `uid` the jailer switches to as it execs the target binary.
    pub uid: Option<u32>,

    // `id` is the unique VM identification string, which may contain alphanumeric
    // characters and hyphens. The maximum id length is currently 64 characters
    pub id: Option<String>,

    // `numa_node` represents the NUMA node the process gets assigned to.
    pub numa_node: Option<usize>,

    // `exec_file` is the path to the Firecracker binary that will be exec-ed by
    // the jailer. The user can provide a path to any binary, but the interaction
    // with the jailer is mostly Firecracker specific.
    pub exec_file: Option<String>,

    // `jailer_bin` specifies the jailer binary to be used for setting up the
    // Firecracker VM jail.
    // If not specified it defaults to "jailer".
    pub jailer_bin: Option<String>,

    // `chroot_base_dir` represents the base folder where chroot jails are built. The
    // default is /srv/jailer
    pub chroot_base_dir: Option<String>,

    // `daemonize` is set to true, call setsid() and redirect STDIN, STDOUT, and
    // STDERR to /dev/null
    pub daemonize: Option<bool>,
}

impl JailerConfig {
    pub fn validate(&self) -> RtckResult<()> {
        match &self.exec_file {
            None => {
                log::error!(
                    "[Rustcracker {}:{}:JailerConfig::validate missing exec_file]",
                    file!(),
                    line!()
                );
                return Err(RtckError::new(
                    RtckErrorClass::ConfigError,
                    "Executable file (firecracker) must be specified in configuration".to_string(),
                ));
            }
            Some(path) => {
                let path = PathBuf::from(path);
                if !path.exists() || !path.is_file() {
                    log::error!(
                        "[Rustcracker {}:{}:JailerConfig::validate missing exec_file]",
                        file!(),
                        line!()
                    );
                    return Err(RtckError::new(
                        RtckErrorClass::ConfigError,
                        "Missing executable file in jailer".to_string(),
                    ));
                }
            }
        }

        match &self.jailer_bin {
            None => {
                log::error!(
                    "[Rustcracker {}:{}:JailerConfig::validate missing jailer_bin]",
                    file!(),
                    line!()
                );
                return Err(RtckError::new(
                    RtckErrorClass::ConfigError,
                    "Jailer binary must be specified in configuration".to_string(),
                ));
            }
            Some(path) => {
                let path = PathBuf::from(path);
                if !path.exists() || !path.is_file() {
                    log::error!(
                        "[Rustcracker {}:{}:JailerConfig::validate missing jailer_bin]",
                        file!(),
                        line!()
                    );
                    return Err(RtckError::new(
                        RtckErrorClass::ConfigError,
                        "Missing jailer binary".to_string(),
                    ));
                }
            }
        }

        match &self.chroot_base_dir {
            None => (),
            Some(path) => log::info!(
                "[Rustcracker {}:{}:JailerConfig::validate using chroot_base_dir = {}]",
                file!(),
                line!(),
                path
            ),
        }

        Ok(())
    }

    pub fn to_vec(&self) -> RtckResult<Vec<u8>> {
        Ok(serde_json::to_vec(&self)?)
    }

    pub fn get_exec_file(&self) -> Option<&String> {
        self.exec_file.as_ref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlobalConfig {
    pub using_jailer: Option<bool>,
    pub jailer_bin: Option<String>,
    pub jailer_config: Option<JailerConfig>,

    pub socket_path: Option<String>,

    pub frck_bin: Option<String>,
    pub frck_config: Option<FirecrackerConfig>,

    // Where to put firecracker exported config
    pub frck_export: Option<String>,
}

impl GlobalConfig {
    pub fn validate(&self) -> RtckResult<()> {
        if self.using_jailer.is_none() || *self.using_jailer.as_ref().unwrap() {
            match &self.jailer_bin {
                Some(path) if !PathBuf::from(path).exists() => {
                    return Err(RtckError::new(
                        RtckErrorClass::ConfigError,
                        "Jailer bin missing".to_string(),
                    ));
                }
                None => {
                    return Err(RtckError::new(
                        RtckErrorClass::ConfigError,
                        "Jailer bin missing".to_string(),
                    ))
                }
                _ => (),
            }

            match &self.jailer_config {
                None => {
                    return Err(RtckError::new(
                        RtckErrorClass::ConfigError,
                        "Using jailer but no jailer config specified".to_string(),
                    ))
                }
                Some(config) => config.validate()?,
            }
        }

        match &self.frck_bin {
            None => {
                return Err(RtckError::new(
                    RtckErrorClass::ConfigError,
                    "Missing firecracker bin entry".to_string(),
                ))
            }
            Some(path) => {
                let path = PathBuf::from(path);
                if !path.exists() || !path.is_file() {
                    return Err(RtckError::new(
                        RtckErrorClass::ConfigError,
                        "Firecracker bin missing".to_string(),
                    ));
                }
            }
        }

        if self.frck_export.is_some() {
            match &self.frck_config {
                None => {
                    return Err(RtckError::new(
                        RtckErrorClass::ConfigError,
                        "Set exporting config but no config".to_string(),
                    ))
                }
                Some(_) => (),
            }
        }

        match &self.socket_path {
            None => {
                return Err(RtckError::new(
                    RtckErrorClass::ConfigError,
                    "Missing socket path entry".to_string(),
                ))
            }
            Some(path) => {
                let path = PathBuf::from(path);
                if path.exists() {
                    return Err(RtckError::new(
                        RtckErrorClass::ConfigError,
                        "Socket already exists".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Export the firecracker config
    pub fn export_config(&self) -> RtckResult<()> {
        match &self.frck_export {
            None => Ok(()),
            Some(path) => Ok(std::fs::write(
                path,
                self.frck_config
                    .as_ref()
                    .ok_or(RtckError::new(
                        RtckErrorClass::ConfigError,
                        "No firecracker config".to_string(),
                    ))?
                    .to_vec()?,
            )?),
        }
    }

    /// Export the firecracker config
    #[cfg(feature = "tokio")]
    pub async fn export_config_async(&self) -> RtckResult<()> {
        match &self.frck_export {
            None => Ok(()),
            Some(path) => Ok(tokio::fs::write(
                path,
                self.frck_config
                    .as_ref()
                    .ok_or(RtckError::new(
                        RtckErrorClass::ConfigError,
                        "No firecracker config".to_string(),
                    ))?
                    .to_vec()?,
            )
            .await?),
        }
    }
}

// Global Config for Firecracker

#[cfg(test)]
mod test {
    use crate::models::{
        balloon::Balloon, drive::Drive, logger::LogLevel,
        machine_configuration::MachineConfiguration, network_interface::NetworkInterface,
    };

    use super::{FirecrackerConfig, GlobalConfig};

    #[test]
    fn test_write_config_consistent() {
        const SAVE_PATH: &'static str = "/tmp/test_firecracker_export_config.json";

        let frck_config = FirecrackerConfig {
            log_path: Some("/var/log/firecracker/vm.log".to_string()),
            log_level: Some(LogLevel::Error),
            log_clear: Some(true),
            metrics_path: Some("/var/metrics/firecracker/metrics".to_string()),
            metrics_clear: Some(true),
            kernel_image_path: Some("/images/ubuntu_22_04.img".to_string()),
            initrd_path: Some("/initrd/initrd_0".to_string()),
            kernel_args: Some("console=ttyS0 reboot=k panic=1 pci=off".to_string()),
            drives: Some(vec![Drive {
                drive_id: "rootfs".to_string(),
                path_on_host: "./ubuntu-22.04.ext4".to_string(),
                is_read_only: false,
                is_root_device: true,
                partuuid: None,
                cache_type: None,
                rate_limiter: None,
                io_engine: None,
                socket: None,
            }]),
            network_interfaces: Some(vec![NetworkInterface {
                guest_mac: Some("06:00:AC:10:00:02".to_string()),
                host_dev_name: "tap0".to_string(),
                iface_id: "net1".to_string(),
                rx_rate_limiter: None,
                tx_rate_limiter: None,
            }]),
            vsock_devices: None,
            machine_cfg: Some(MachineConfiguration {
                cpu_template: None,
                ht_enabled: None,
                mem_size_mib: 256,
                track_dirty_pages: None,
                vcpu_count: 8,
            }),
            vmid: Some("test_machine".to_string()),
            net_ns: Some("mynetns".to_string()),
            network_clear: Some(true),
            seccomp_level: None,
            mmds_address: None,
            balloon: Some(Balloon {
                amount_mib: 64,
                deflate_on_oom: true,
                stats_polling_interval_s: None,
            }),
            init_metadata: Some("This is initial metadata".to_string()),
        };

        let config = GlobalConfig {
            using_jailer: Some(false),
            jailer_bin: None,
            jailer_config: None,
            socket_path: Some("/tmp/firecracker.sock".to_string()),
            frck_bin: Some("/usr/bin/firecracker".to_string()),
            frck_config: Some(frck_config),
            frck_export: Some(SAVE_PATH.to_string()),
        };

        config.export_config().expect("Fail to export config");

        let vec = std::fs::read(SAVE_PATH).expect("Fail to read config from file");

        let config_: FirecrackerConfig =
            serde_json::from_slice(&vec).expect("Fail to deserialize the config");

        assert_eq!(config.frck_config, Some(config_));
    }
}
