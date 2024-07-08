use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{models::*, RtckError, RtckResult};

/// Firecracker configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FirecrackerConfig {
    // logger defines the logger for microVM.
    pub logger: Option<logger::Logger>,

    // metrics defines the file path where the Firecracker metrics
    // is located.
    pub metrics: Option<metrics::Metrics>,

    // boot_source defines the kernel image path, initrd path and kernel args.
    pub boot_source: Option<boot_source::BootSource>,

    // drives specifies BlockDevices that should be made available to the
    // microVM.
    pub drives: Option<Vec<drive::Drive>>,

    // network_interfaces specifies the tap devices that should be made available
    // to the microVM.
    pub network_interfaces: Option<Vec<network_interface::NetworkInterface>>,

    // vsock_devices specifies the vsock devices that should be made available to
    // the microVM.
    pub vsock_devices: Option<Vec<vsock::Vsock>>,

    // cpu_config defines the CPU configuration of microVM.
    pub cpu_config: Option<cpu_template::CPUConfig>,

    // machine_cfg represents the firecracker microVM process configuration
    pub machine_config: Option<machine_configuration::MachineConfiguration>,

    // (Optional) vmid is a unique identifier for this VM. It's set to a
    // random uuid if not provided by the user. It's used to set Firecracker's instance ID.
    // If CNI configuration is provided as part of NetworkInterfaces,
    // the vmid is used to set CNI ContainerID and create a network namespace path.
    pub vmid: Option<String>,

    // net_ns represents the path to a network namespace handle. If present, the
    // application will use this to join the associated network namespace
    pub net_ns: Option<String>,

    // mmds_address is IPv4 address used by guest applications when issuing requests to MMDS.
    // It is possible to use a valid IPv4 link-local address (169.254.0.0/16).
    // If not provided, the default address (169.254.169.254) will be used.
    pub mmds_address: Option<std::net::Ipv4Addr>,

    // balloon is Balloon device that is to be put to the machine
    pub balloon: Option<balloon::Balloon>,

    // entropy_device defines the entropy device used by microVM.
    pub entropy_device: Option<entropy_device::EntropyDevice>,

    // init_metadata is initial metadata that is to be assigned to the machine
    pub init_metadata: Option<String>,
}

impl FirecrackerConfig {
    pub fn validate(&self) -> RtckResult<()> {
        match &self.logger {
            None => (),
            Some(logger) => {
                let path = PathBuf::from(&logger.log_path);
                if path.exists() {
                    return Err(RtckError::Config("logger path occupied".to_string()));
                }
            }
        }

        match &self.metrics {
            None => (),
            Some(metrics) => {
                let path = PathBuf::from(&metrics.metrics_path);
                if path.exists() {
                    return Err(RtckError::Config("metrics path occupied".to_string()));
                }
            }
        }

        match &self.boot_source {
            None => log::warn!("[FirecrackerConfig::validate must designate boot source]"),
            Some(boot_source) => {
                let path = PathBuf::from(&boot_source.kernel_image_path);
                if !path.exists() || !path.is_file() {
                    return Err(RtckError::Config("kernel image file missing".to_string()));
                }
            }
        }

        Ok(())
    }

    pub fn to_vec(&self) -> RtckResult<Vec<u8>> {
        serde_json::to_vec(&self)
            .map_err(|_| RtckError::Encode("firecracker config to vec".to_string()))
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
                return Err(RtckError::Config("no executable file".to_string()));
            }
            Some(path) => {
                // forced by firecracker specification
                if !path.contains("firecracker") {
                    return Err(RtckError::Config(
                        "executable path must contain `firecracker`".to_string(),
                    ));
                }
                let path = PathBuf::from(path);
                if !path.exists() || !path.is_file() {
                    log::error!(
                        "[Rustcracker {}:{}:JailerConfig::validate missing exec_file]",
                        file!(),
                        line!()
                    );
                    return Err(RtckError::Config(
                        "no executable file with jailer".to_string(),
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
                return Err(RtckError::Config(
                    "jailer binary must be specified in configuration".to_string(),
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
                    return Err(RtckError::Config("missing jailer binary".to_string()));
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
        serde_json::to_vec(&self).map_err(|_| RtckError::Encode("jailer config to vec".to_string()))
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

    // Where to put socket, default to None, and Local will allocate one for you
    pub socket_path: Option<String>,

    // Where to put lock file, default to None, and Local will allocate one for you
    pub lock_path: Option<String>,

    pub frck_bin: Option<String>,
    pub frck_config: Option<FirecrackerConfig>,

    // Where to put firecracker exported config
    pub frck_export_path: Option<String>,

    // log_clear defines whether rustcracker should remove log files after microVM
    // was removed. Default to false.
    pub log_clear: Option<bool>,

    // metrics_clear defines whether rustcracker should remove log files after microVM
    // was removed. Default to false.
    pub metrics_clear: Option<bool>,

    // network_clear defines whether rustcracker should clear networking
    // after the microVM is removed. Default to false.
    pub network_clear: Option<bool>,

    // seccomp_level specifies whether seccomp filters should be installed and how
    // restrictive they should be. Possible values are:
    //
    //	0 : (default): disabled.
    //	1 : basic filtering. This prohibits syscalls not whitelisted by Firecracker.
    //	2 : advanced filtering. This adds further checks on some of the
    //			parameters of the allowed syscalls.
    pub seccomp_level: Option<usize>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            using_jailer: None,
            jailer_bin: None,
            jailer_config: None,
            socket_path: None,
            lock_path: None,
            frck_bin: None,
            frck_config: None,
            frck_export_path: None,
            log_clear: None,
            metrics_clear: None,
            network_clear: None,
            seccomp_level: None,
        }
    }
}

impl GlobalConfig {
    pub fn validate(&self) -> RtckResult<()> {
        if self.using_jailer.is_none() || *self.using_jailer.as_ref().unwrap() {
            match &self.jailer_bin {
                Some(path) if !PathBuf::from(path).exists() => {
                    return Err(RtckError::Config("jailer bin missing".to_string()));
                }
                None => return Err(RtckError::Config("jailer bin missing".to_string())),
                _ => (),
            }

            match &self.jailer_config {
                None => return Err(RtckError::Config("no jailer config".to_string())),
                Some(config) => config.validate()?,
            }
        }

        match &self.frck_bin {
            None => {
                return Err(RtckError::Config(
                    "missing firecracker bin entry".to_string(),
                ))
            }
            Some(path) => {
                let path = PathBuf::from(path);
                if !path.exists() || !path.is_file() {
                    return Err(RtckError::Config("firecracker bin not found".to_string()));
                }
            }
        }

        if self.frck_export_path.is_some() {
            match &self.frck_config {
                None => return Err(RtckError::Config("no config to export".to_string())),
                Some(_) => (),
            }
        }

        match &self.socket_path {
            None => return Err(RtckError::Config("missing socket path entry".to_string())),
            Some(path) => {
                let path = PathBuf::from(path);
                if path.exists() {
                    return Err(RtckError::Config("socket path occupied".to_string()));
                }
            }
        }

        Ok(())
    }

    /// Export the firecracker config
    pub fn export_config(&self) -> RtckResult<()> {
        match &self.frck_export_path {
            None => Ok(()),
            Some(path) => std::fs::write(
                path,
                self.frck_config
                    .as_ref()
                    .ok_or(RtckError::Config("no firecracker config".to_string()))?
                    .to_vec()?,
            )
            .map_err(|e| RtckError::FilesysIO(format!("when exporting config, {}", e.to_string()))),
        }
    }

    /// Export the firecracker config
    pub async fn export_config_async(&self) -> RtckResult<()> {
        match &self.frck_export_path {
            None => Ok(()),
            Some(path) => tokio::fs::write(
                path,
                self.frck_config
                    .as_ref()
                    .ok_or(RtckError::Config("no firecracker config".to_string()))?
                    .to_vec()?,
            )
            .await
            .map_err(|_| RtckError::FilesysIO("exporting config".to_string())),
        }
    }
}

// Global Config for Firecracker

#[cfg(test)]
mod test {
    use crate::{
        config::boot_source,
        models::{
            balloon::Balloon,
            drive::Drive,
            logger::{self, LogLevel},
            machine_configuration::MachineConfiguration,
            metrics,
            network_interface::NetworkInterface,
        },
    };

    use super::{FirecrackerConfig, GlobalConfig};

    #[test]
    fn test_write_config_consistent() {
        const SAVE_PATH: &'static str = "/tmp/test_firecracker_export_config.json";

        let frck_config = FirecrackerConfig {
            logger: Some(logger::Logger {
                log_path: "/var/log/firecracker/vm.log".to_string(),
                level: Some(LogLevel::Error),
                show_level: None,
                show_log_origin: Some(true),
                module: None,
            }),
            metrics: Some(metrics::Metrics {
                metrics_path: "/var/metrics/firecracker/metrics".to_string(),
            }),
            boot_source: Some(boot_source::BootSource {
                boot_args: Some("console=ttyS0 reboot=k panic=1 pci=off".to_string()),
                initrd_path: None,
                kernel_image_path: "/images/ubuntu_22_04.img".to_string(),
            }),
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
            cpu_config: None,
            machine_config: Some(MachineConfiguration {
                cpu_template: None,
                ht_enabled: None,
                mem_size_mib: 256,
                track_dirty_pages: None,
                vcpu_count: 8,
            }),
            vmid: Some("test_machine".to_string()),
            net_ns: Some("mynetns".to_string()),
            mmds_address: None,
            balloon: Some(Balloon {
                amount_mib: 64,
                deflate_on_oom: true,
                stats_polling_interval_s: None,
            }),
            entropy_device: None,
            init_metadata: Some("This is initial metadata".to_string()),
        };

        let config = GlobalConfig {
            using_jailer: Some(false),
            jailer_bin: None,
            jailer_config: None,
            socket_path: Some("/tmp/firecracker.sock".to_string()),
            lock_path: None,
            frck_bin: Some("/usr/bin/firecracker".to_string()),
            frck_config: Some(frck_config),
            frck_export_path: Some(SAVE_PATH.to_string()),
            log_clear: Some(false),
            metrics_clear: Some(false),
            network_clear: Some(false),
            seccomp_level: None,
        };

        config.export_config().expect("Fail to export config");

        let vec = std::fs::read(SAVE_PATH).expect("Fail to read config from file");

        let config_: FirecrackerConfig =
            serde_json::from_slice(&vec).expect("Fail to deserialize the config");

        assert_eq!(config.frck_config, Some(config_));
    }
}
