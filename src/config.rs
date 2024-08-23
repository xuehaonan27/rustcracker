use crate::{models::*, RtckError, RtckResult};
use log::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for a microVM instance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MicroVMConfig {
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

impl MicroVMConfig {
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

/// Configuration for `jailer`.
/// Needed when using jailer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JailerConfig {
    // `gid` the jailer switches to as it execs the target binary.
    pub gid: Option<u32>,

    // `uid` the jailer switches to as it execs the target binary.
    pub uid: Option<u32>,

    // `id` is the unique VM identification string, which may contain alphanumeric
    // characters and hyphens. The maximum id length is currently 64 characters
    // pub id: Option<String>,

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
    /// instance id.
    /// If set to None, then the hypervisor will allocate a random one for you.
    /// So if you want to make sure which hypervisor you are running now,
    /// you'd better assign a value to this field :)
    pub id: Option<String>,

    /// launch timeout
    pub launch_timeout: u64,

    /// intervals in seconds for hypervisor polling the status
    /// of the microVM it holds when waiting for the user to
    /// give up the microVM.
    /// Default to 10 (seconds).
    /// Could be set bigger to avoid large amount of log being produced.
    pub poll_status_secs: u64,

    /// using jailer?
    pub using_jailer: Option<bool>,

    /// path to firecracker binary
    pub frck_bin: Option<String>,

    /// path to jailer binary (if using jailer)
    pub jailer_bin: Option<String>,

    /// jailer configuration (if using jailer)
    pub jailer_config: Option<JailerConfig>,

    /// where to put firecracker exported config for `--config`
    pub frck_export_path: Option<String>,

    /// where to put socket, default to None, and hypervisor will allocate one for you.
    /// When using jailer, default value "run/firecracker.socket" is used since this
    /// name is impossible to conflict with other sockets when using jailer.
    /// When not using jailer, default value would be "/run/firecracker-<id>.socket",
    /// where <id> is the instance id of your hypervisor.
    pub socket_path: Option<String>,

    /// socket retrying times, default to 3 times
    pub socket_retry: usize,

    /// where to put lock file, default to None, and Local will allocate one for you
    pub lock_path: Option<String>,

    /// hypervisor log path
    /// path inside jailer seen by firecracker (when using jailer)
    pub log_path: Option<String>,

    /// log_clear defines whether rustcracker should remove log files after microVM
    /// instance is removed from hypervisor. Default to false.
    pub log_clear: Option<bool>,

    /// hypervisor metrics path
    /// path inside jailer seen by firecracker (when using jailer)
    pub metrics_path: Option<String>,

    /// metrics_clear defines whether rustcracker should remove log files after microVM
    /// instance is removed from hypervisor. Default to false.
    pub metrics_clear: Option<bool>,

    /// network_clear defines whether rustcracker should clear networking after microVM
    /// instance is removed from hypervisor. Default to false.
    pub network_clear: Option<bool>,

    /// seccomp_level specifies whether seccomp filters should be installed and how
    /// restrictive they should be. Possible values are:
    ///
    ///	0 : (default): disabled.
    ///	1 : basic filtering. This prohibits syscalls not whitelisted by Firecracker.
    ///	2 : advanced filtering. This adds further checks on some of the
    ///			parameters of the allowed syscalls.
    pub seccomp_level: Option<usize>,

    // /// redirect stdout here
    // pub stdout_to: Option<String>,

    // /// redirect stderr here
    // pub stderr_to: Option<String>,
    /// clear jailer directory, default to false
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
            // stdout_to: None,
            // stderr_to: None,
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
}

/// Configuration combined for fast path microVM creation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlobalConfig {
    pub frck_config: Option<MicroVMConfig>,
    pub hv_config: Option<HypervisorConfig>,
}
