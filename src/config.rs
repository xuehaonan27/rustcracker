use crate::{models::*, RtckError, RtckResult};
use log::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for a microVM instance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MicroVMConfig {
    /// The logger for microVM.
    pub logger: Option<logger::Logger>,

    /// The file path where the Firecracker metrics is located.
    pub metrics: Option<metrics::Metrics>,

    /// Kernel image path, initrd path (optional) and kernel args.
    pub boot_source: Option<boot_source::BootSource>,

    /// Block devices that should be made available to the microVM.
    pub drives: Option<Vec<drive::Drive>>,

    /// Tap devices that should be made available to the microVM.
    pub network_interfaces: Option<Vec<network_interface::NetworkInterface>>,

    /// Vsock devices that should be made available to the microVM.
    pub vsock_devices: Option<Vec<vsock::Vsock>>,

    /// CPU configuration of microVM.
    pub cpu_config: Option<cpu_template::CPUConfig>,

    /// Firecracker microVM process configuration.
    pub machine_config: Option<machine_configuration::MachineConfiguration>,

    /// (Optional) vmid is a unique identifier for this VM. It's set to a
    /// random uuid if not provided by the user. It's used to set Firecracker's instance ID.
    pub vmid: Option<String>,

    /// The path to a network namespace handle. If present, the
    /// application will use this to join the associated network namespace
    pub net_ns: Option<String>,

    /// IPv4 address used by guest applications when issuing requests to MMDS.
    pub mmds_address: Option<std::net::Ipv4Addr>,

    /// Balloon device that is to be put to the machine.
    pub balloon: Option<balloon::Balloon>,

    /// The entropy device.
    pub entropy_device: Option<entropy_device::EntropyDevice>,

    /// Initial metadata that is to be assigned to the machine.
    pub init_metadata: Option<String>,
}

impl Default for MicroVMConfig {
    fn default() -> Self {
        Self {
            logger: None,
            metrics: None,
            boot_source: None,
            drives: None,
            network_interfaces: None,
            vsock_devices: None,
            cpu_config: None,
            machine_config: None,
            vmid: None,
            net_ns: None,
            mmds_address: None,
            balloon: None,
            entropy_device: None,
            init_metadata: None,
        }
    }
}

impl MicroVMConfig {
    pub fn validate(&self) -> RtckResult<()> {
        match &self.logger {
            None => (),
            Some(logger) => {
                let path = PathBuf::from(&logger.log_path);
                if path.exists() {
                    let msg = "Logger path occupied";
                    error!("{msg}");
                    return Err(RtckError::Config(msg.into()));
                }
            }
        }

        match &self.metrics {
            None => (),
            Some(metrics) => {
                let path = PathBuf::from(&metrics.metrics_path);
                if path.exists() {
                    let msg = "Metrics path occupied";
                    error!("{msg}");
                    return Err(RtckError::Config(msg.into()));
                }
            }
        }

        match &self.boot_source {
            None => log::warn!("[FirecrackerConfig::validate must designate boot source]"),
            Some(boot_source) => {
                let path = PathBuf::from(&boot_source.kernel_image_path);
                if !path.exists() || !path.is_file() {
                    let msg = format!("Kernel image file not found at {path:?}");
                    error!("{msg}");
                    return Err(RtckError::Config(msg));
                }
            }
        }

        Ok(())
    }
}

/// Configuration for `jailer`.
/// Needed when using jailer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JailerConfig {
    /// `gid` the jailer switches to as it execs the target binary.
    pub gid: Option<u32>,

    /// `uid` the jailer switches to as it execs the target binary.
    pub uid: Option<u32>,

    /// `numa_node` represents the NUMA node the process gets assigned to.
    pub numa_node: Option<usize>,

    /// `exec_file` is the path to the Firecracker binary that will be exec-ed by
    /// the jailer. The user can provide a path to any binary, but the interaction
    /// with the jailer is mostly Firecracker specific.
    pub exec_file: Option<String>,

    /// `jailer_bin` specifies the jailer binary to be used for setting up the
    /// Firecracker VM jail.
    /// If not specified it defaults to "jailer".
    pub jailer_bin: Option<String>,

    /// `chroot_base_dir` represents the base folder where chroot jails are built. The
    /// default is /srv/jailer
    pub chroot_base_dir: Option<String>,

    /// `daemonize` is set to true, call setsid() and redirect STDIN, STDOUT, and
    /// STDERR to /dev/null
    pub daemonize: Option<bool>,
}

impl Default for JailerConfig {
    fn default() -> Self {
        Self {
            gid: None,
            uid: None,
            numa_node: None,
            exec_file: None,
            jailer_bin: None,
            chroot_base_dir: None,
            daemonize: None,
        }
    }
}

impl JailerConfig {
    pub fn validate(&self) -> RtckResult<()> {
        match &self.exec_file {
            None => {
                let msg = "Firecracker executable file not configured";
                error!("{msg}");
                return Err(RtckError::Config(msg.into()));
            }
            Some(path) => {
                // forced by firecracker specification
                if !path.contains("firecracker") {
                    let msg = "Firecracker executable path must contain `firecracker`";
                    error!("{msg}");
                    return Err(RtckError::Config(msg.into()));
                }
                let path = PathBuf::from(path);
                if !path.exists() || !path.is_file() {
                    let msg = format!("Firecracker executable file not found at {path:?}");
                    error!("{msg}");
                    return Err(RtckError::Config(msg));
                }
            }
        }

        match &self.jailer_bin {
            None => {
                let msg = "Jailer binary path not configured";
                error!("{msg}");
                return Err(RtckError::Config(msg.into()));
            }
            Some(path) => {
                let path = PathBuf::from(path);
                if !path.exists() || !path.is_file() {
                    let msg = format!("Jailer binary not fount at {path:?}");
                    error!("{msg}");
                    return Err(RtckError::Config(msg));
                }
            }
        }

        match &self.chroot_base_dir {
            None => (),
            Some(path) => info!("Using {path} as jailer chroot base directory"),
        }

        trace!("Jailer configuration validated");

        Ok(())
    }
}

/// Configuration relative to a hypervisor instance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HypervisorConfig {
    /// Instance id.
    /// If set to None, then the hypervisor will allocate a random one for you.
    /// So if you want to make sure which hypervisor you are running now,
    /// you'd better assign a value to this field :)
    pub id: Option<String>,

    /// Launch timeout
    pub launch_timeout: u64,

    /// Interval in seconds for hypervisor polling the status
    /// of the microVM it holds when waiting for the user to
    /// give up the microVM.
    /// Default to 10 (seconds).
    /// Could be set bigger to avoid large amount of log being produced.
    pub poll_status_secs: u64,

    /// Whether jailer should be used or not
    pub using_jailer: Option<bool>,

    /// Path to firecracker binary
    pub frck_bin: Option<String>,

    /// Path to jailer binary (if using jailer)
    pub jailer_bin: Option<String>,

    /// Jailer configuration (if using jailer)
    pub jailer_config: Option<JailerConfig>,

    /// Where to put firecracker exported config for `--config`
    pub frck_export_path: Option<String>,

    /// Where to put socket, default to None, and hypervisor will allocate one for you.
    /// When using jailer, default value "run/firecracker.socket" is used since this
    /// name is impossible to conflict with other sockets when using jailer.
    /// When not using jailer, default value would be "/run/firecracker-<id>.socket",
    /// where <id> is the instance id of your hypervisor.
    pub socket_path: Option<String>,

    /// Socket retrying times, default to 3 times
    pub socket_retry: usize,

    /// Where to put lock file, default to None, and Local will allocate one for you
    pub lock_path: Option<String>,

    /// Hypervisor log path
    /// path inside jailer seen by firecracker (when using jailer)
    pub log_path: Option<String>,

    /// Whether log files should be removed after microVM
    /// instance is removed. Default to false.
    pub log_clear: Option<bool>,

    /// Hypervisor metrics path
    /// path inside jailer seen by firecracker (when using jailer)
    pub metrics_path: Option<String>,

    /// Whether metrics files should be removed after microVM
    /// instance is removed. Default to false.
    pub metrics_clear: Option<bool>,

    /// Whether networking should be cleared after microVM
    /// instance is removed from hypervisor. Default to false.
    pub network_clear: Option<bool>,

    /// Whether seccomp filters should be installed and how restrictive
    /// they should be. Possible values are:
    ///
    ///	0 : (default): disabled.
    ///	1 : basic filtering. This prohibits syscalls not whitelisted by Firecracker.
    ///	2 : advanced filtering. This adds further checks on some of the
    ///			parameters of the allowed syscalls.
    pub seccomp_level: Option<usize>,

    /// Whether jailer working directory should be removed after
    /// microVM instance is removed, default to false
    pub clear_jailer: Option<bool>,
}

impl Default for HypervisorConfig {
    fn default() -> Self {
        Self {
            id: None,
            launch_timeout: 3,
            using_jailer: None,
            jailer_bin: None,
            jailer_config: None,
            frck_export_path: None,
            socket_path: None,
            socket_retry: 3,
            lock_path: None,
            frck_bin: None,
            log_path: None,
            log_clear: None,
            metrics_path: None,
            metrics_clear: None,
            network_clear: None,
            seccomp_level: None,
            clear_jailer: None,
            poll_status_secs: 10,
        }
    }
}

impl HypervisorConfig {
    pub fn validate(&self) -> RtckResult<()> {
        if let Some(true) = self.using_jailer {
            match &self.jailer_bin {
                Some(path) if !PathBuf::from(path).exists() => {
                    let msg = "Jailer binary path not configured";
                    error!("{msg}");
                    return Err(RtckError::Config(msg.into()));
                }
                None => {
                    let msg = "Jailer binary path not configured";
                    error!("{msg}");
                    return Err(RtckError::Config(msg.into()));
                }
                _ => (),
            }

            match &self.jailer_config {
                None => {
                    let msg = "No jailer config";
                    error!("{msg}");
                    return Err(RtckError::Config(msg.into()));
                }
                Some(config) => config.validate()?,
            }
        } else {
            match &self.socket_path {
                None => {
                    let msg = "Socket path not configured";
                    error!("{msg}");
                    return Err(RtckError::Config(msg.into()));
                }
                Some(path) => {
                    let path = PathBuf::from(path);
                    if path.exists() {
                        let msg = "Socket path occupied";
                        error!("{msg}");
                        return Err(RtckError::Config(msg.into()));
                    }
                }
            }
        }

        match &self.frck_bin {
            None => {
                let msg = "Firecracker binary path not configured";
                error!("{msg}");
                return Err(RtckError::Config(msg.into()));
            }
            Some(path) => {
                let path = PathBuf::from(path);
                if !path.exists() || !path.is_file() {
                    let msg = format!("Firecracker binary not found, provided path: {path:#?}");
                    error!("{msg}");
                    return Err(RtckError::Config(msg));
                }
            }
        }

        trace!("Hypervisor configuration validated");
        Ok(())
    }

    pub(super) fn jailer_config(&mut self) -> Option<&mut JailerConfig> {
        self.jailer_config.as_mut()
    }
}
