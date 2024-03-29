use std::{ffi::OsStr, path::PathBuf};

use log::{debug, error, info, warn};
use nix::{fcntl, sys::stat::Mode, unistd};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::{
    components::command_builder::VMMCommandBuilder,
    model::{
        balloon::Balloon,
        balloon_stats::BalloonStatistics,
        balloon_stats_update::BalloonStatsUpdate,
        balloon_update::BalloonUpdate,
        boot_source::BootSource,
        drive::Drive,
        firecracker_version::FirecrackerVersion,
        full_vm_configuration::FullVmConfiguration,
        instance_action_info::InstanceActionInfo,
        instance_info::InstanceInfo,
        logger::{LogLevel, Logger},
        machine_configuration::MachineConfiguration,
        metrics::Metrics,
        mmds_config::MmdsConfig,
        network_interface::NetworkInterface,
        partial_drive::PartialDrive,
        partial_network_interface::PartialNetworkInterface,
        rate_limiter::RateLimiterSet,
        snapshot_create_params::SnapshotCreateParams,
        snapshot_load_params::SnapshotLoadParams,
        vm::{VM_STATE_PAUSED, VM_STATE_RESUMED},
        vsock::Vsock,
    },
    utils::*,
};

use super::{agent::Agent, command_builder::JailerCommandBuilder, jailer::JailerConfig};

type SeccompLevelValue = usize;

pub enum SeccompLevel {
    Disable,
    Basic,
    Advanced,
}

// SeccompLevelDisable is the default value.
const SECCOMP_LEVEL_DISABLE: SeccompLevelValue = 0;

// SeccompLevelBasic prohibits syscalls not whitelisted by Firecracker.
const SECCOMP_LEVEL_BASIC: SeccompLevelValue = 1;

// SeccompLevelAdvanced adds further checks on some of the parameters of the
// allowed syscalls.
const SECCOMP_LEVEL_ADVANCED: SeccompLevelValue = 2;

/// Config is a collection of user-configurable VMM settings
/// describe the microVM
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    /// socket_path defines the file path where the Firecracker control socket
    /// should be created.
    pub socket_path: Option<PathBuf>,

    /// log_path defines the file path where the Firecracker log is located.
    /// will be disabled if log_fifo is set
    pub log_path: Option<PathBuf>,

    /// log_fifo defines the file path where the Firecracker log named-pipe should
    /// be located.
    pub log_fifo: Option<PathBuf>,

    /// log_level defines the verbosity of Firecracker logging.  Valid values are
    /// "Error", "Warning", "Info", and "Debug", and are case-sensitive.
    pub log_level: Option<LogLevel>,

    /// log_clear defines whether rustcracker should remove log files after microVM
    /// was removed. Default to false.
    pub log_clear: Option<bool>,

    /// metrics_path defines the file path where the Firecracker metrics
    /// is located.
    /// will be disabled if metrics_fifo is set
    pub metrics_path: Option<PathBuf>,

    /// metrics_fifo defines the file path where the Firecracker metrics
    /// named-pipe should be located.
    pub metrics_fifo: Option<PathBuf>,

    /// metrics_clear defines whether rustcracker should remove log files after microVM
    /// was removed. Default to false.
    pub metrics_clear: Option<bool>,

    /// kernel_image_path defines the file path where the kernel image is located.
    /// The kernel image must be an uncompressed ELF image.
    pub kernel_image_path: Option<PathBuf>,

    /// initrd_path defines the file path where initrd image is located.
    /// This parameter is optional.
    pub initrd_path: Option<PathBuf>,

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
    // pub fifo_log_writer: Option<PathBuf>,

    /// vsock_devices specifies the vsock devices that should be made available to
    /// the microVM.
    pub vsock_devices: Option<Vec<Vsock>>,

    /// machine_cfg represents the firecracker microVM process configuration
    pub machine_cfg: Option<MachineConfiguration>,

    /// disable_validation allows for easier mock testing by disabling the
    /// validation of configuration performed by the SDK(crate).
    pub disable_validation: bool,

    /// enable_jailer judge whether jailer should be used
    /// Default to false
    pub enable_jailer: bool,

    /// jailer_cfg is configuration specific for the jailer process.
    pub jailer_cfg: Option<JailerConfig>,

    /// (Optional) vmid is a unique identifier for this VM. It's set to a
    /// random uuid if not provided by the user. It's used to set Firecracker's instance ID.
    /// If CNI configuration is provided as part of NetworkInterfaces,
    /// the vmid is used to set CNI ContainerID and create a network namespace path.
    pub vmid: Option<String>,

    /// net_ns represents the path to a network namespace handle. If present, the
    /// application will use this to join the associated network namespace
    pub net_ns: Option<PathBuf>,

    /// network_clear defines whether rustcracker should clear networking
    /// after the microVM is removed. Default to false.
    pub network_clear: Option<bool>,

    /// ForwardSignals is an optional list of signals to catch and forward to
    /// firecracker. If not provided, the default signals will be used.
    pub forward_signals: Option<Vec<u8>>,

    /// seccomp_level specifies whether seccomp filters should be installed and how
    /// restrictive they should be. Possible values are:
    ///
    ///	0 : (default): disabled.
    ///	1 : basic filtering. This prohibits syscalls not whitelisted by Firecracker.
    ///	2 : advanced filtering. This adds further checks on some of the
    ///			parameters of the allowed syscalls.
    pub seccomp_level: Option<SeccompLevelValue>,

    /// mmds_address is IPv4 address used by guest applications when issuing requests to MMDS.
    /// It is possible to use a valid IPv4 link-local address (169.254.0.0/16).
    /// If not provided, the default address (169.254.169.254) will be used.
    pub mmds_address: Option<std::net::Ipv4Addr>,

    /// balloon is Balloon device that is to be put to the machine
    pub balloon: Option<Balloon>,

    /// init_metadata is initial metadata that is to be assigned to the machine
    pub init_metadata: Option<String>,

    /// stdout specifies the stdout to use when spawning the firecracker.
    /// pub(crate) stdout: Option<std::process::Stdio>,
    pub stdout: Option<StdioTypes>,

    /// stderr specifies the IO writer for STDERR to use when spawning the jailer.
    pub stderr: Option<StdioTypes>,

    /// stdin specifies the IO reader for STDIN to use when spawning the jailer.
    pub stdin: Option<StdioTypes>,

    /// agent_init_timeout is the init timeout (in secs) for launching a firecracker
    /// UDS agent, which could be overwritten by setting environment variable
    /// `FIRECRACKER_INIT_TIMEOUT_ENV`
    /// default to 3.0 (if set to None)
    pub agent_init_timeout: Option<f64>,

    /// agent_request_timeout is the request timeout (in secs) for communicating
    /// with firecracker, which could be overwritten by setting environment variable
    /// `FIRECRACKER_REQUEST_TIMEOUT_ENV`
    /// default to 3.0 (if set to None)
    pub agent_request_timeout: Option<f64>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            socket_path: None,
            log_path: None,
            log_fifo: None,
            log_level: None,
            log_clear: None,
            metrics_path: None,
            metrics_fifo: None,
            metrics_clear: None,
            kernel_image_path: None,
            initrd_path: None,
            kernel_args: None,
            drives: None,
            network_interfaces: None,
            vsock_devices: None,
            machine_cfg: None,
            disable_validation: false,
            enable_jailer: false,
            jailer_cfg: None,
            vmid: None,
            net_ns: None,
            network_clear: None,
            seccomp_level: None,
            mmds_address: None,
            forward_signals: None,
            balloon: None,
            init_metadata: None,
            stderr: None,
            stdout: None,
            stdin: None,
            agent_init_timeout: None,
            agent_request_timeout: None,
        }
    }
}

impl Config {
    /// called by ConfigValidationHandler
    pub(super) fn validate(&self) -> Result<(), MachineError> {
        if self.disable_validation {
            return Ok(());
        }

        if self.kernel_image_path.is_none() {
            return Err(MachineError::Validation(
                "no kernel image path provided".to_string(),
            ));
        } else if let Err(e) = std::fs::metadata(self.kernel_image_path.as_ref().unwrap()) {
            return Err(MachineError::Validation(format!(
                "failed to stat kernel image path, {:#?}: {}",
                self.kernel_image_path,
                e.to_string()
            )));
        }

        // initrd_path being None is allowed
        if self.initrd_path.is_some() {
            if let Err(e) = std::fs::metadata(self.initrd_path.as_ref().unwrap()) {
                return Err(MachineError::Validation(format!(
                    "failed to stat initrd image path, {:#?}: {}",
                    self.initrd_path,
                    e.to_string()
                )));
            }
        }

        if self.drives.is_some() {
            for drive in self.drives.as_ref().unwrap() {
                if drive.is_root_device() {
                    let root_path = drive.get_path_on_host();

                    if let Err(e) = std::fs::metadata(&root_path) {
                        return Err(MachineError::Validation(format!(
                            "failed to stat host drive path, {:#?}: {}",
                            root_path,
                            e.to_string()
                        )));
                    }

                    break;
                }
            }
        }

        // Check the non-existence of some files, like socket:
        if self.socket_path.is_none() {
            return Err(MachineError::Validation(
                "no socket path provided".to_string(),
            ));
        } else if let Ok(_) = std::fs::metadata(self.socket_path.as_ref().unwrap()) {
            return Err(MachineError::Validation(format!(
                "socket {:#?} already exists",
                self.socket_path
            )));
        } else {
            // create socket parent dir
            let socket_dir_parent = self.socket_path.as_ref().unwrap().parent();
            if socket_dir_parent.is_none() {
                return Err(MachineError::Validation(format!(
                    "invalid directory where the socket is to be generated: {}",
                    self.socket_path.as_ref().unwrap().display()
                )));
            }
            std::fs::create_dir_all(socket_dir_parent.unwrap()).map_err(|e| {
                MachineError::FileCreation(format!(
                    "fail to create socket parent directory {}: {}",
                    socket_dir_parent.as_ref().unwrap().display(),
                    e.to_string()
                ))
            })?;
        }

        // validate machine configuration
        if self.machine_cfg.is_none() {
            return Err(MachineError::Validation(
                "no machine configuration provided".to_string(),
            ));
        } else {
            self.machine_cfg.as_ref().unwrap().validate()?;
        }

        if self.drives.is_some() {
            for drive in self.drives.as_ref().unwrap() {
                drive.validate()?;
            }
        }

        // network interfaces are validated in fn Config::validate_network

        if self.vsock_devices.is_some() {
            for dev in self.vsock_devices.as_ref().unwrap() {
                dev.validate()?;
            }
        }

        Ok(())
    }

    // called by NetworkConfigValidationHandler
    pub(super) fn validate_network(&self) -> Result<(), MachineError> {
        if self.disable_validation {
            return Ok(());
        }
        if self.net_ns.is_none() {
            return Err(MachineError::Validation(
                "no network namespace provided".to_string(),
            ));
        }

        if self.network_interfaces.is_none() || self.network_interfaces.as_ref().unwrap().len() == 0
        {
            return Err(MachineError::Validation(
                "no network interface provided".to_string(),
            ));
        }

        // let s: KernelArgs;
        // if self.kernel_args.is_some() {
        //     s = KernelArgs::from(self.kernel_args.as_ref().unwrap().to_owned());
        // } else {
        //     s = KernelArgs(std::collections::HashMap::new());
        // }

        // for iface in self.network_interfaces.as_ref().unwrap() {
        //     iface.validate()?;
        // }

        Ok(())
    }

    pub fn validate_jailer_config(&self) -> Result<(), MachineError> {
        if self.disable_validation {
            return Ok(());
        }

        if self.jailer_cfg.is_none() {
            return Err(MachineError::ArgWrong("Missing JailerConfig".to_string()))
        }

        let mut has_root = self.initrd_path.is_some();
        for drive in self.drives.as_ref().unwrap() {
            if drive.is_root_device() {
                has_root = true;
            }
        }

        if !has_root {
            error!("A root drive must be present in the drive list");
            return Err(MachineError::Validation(
                "A root drive must be present in the drive list".to_string(),
            ));
        }

        if self.jailer_cfg.as_ref().unwrap().exec_file.is_none() {
            error!("exec file must be specified when using jailer mode");
            return Err(MachineError::Validation(
                "exec file must be specified when using jailer mode".to_string(),
            ));
        }

        if self.jailer_cfg.as_ref().unwrap().id.is_none()
            || self.jailer_cfg.as_ref().unwrap().id.as_ref().unwrap().len() == 0
        {
            error!("id must be specified when using jailer mode");
            return Err(MachineError::Validation(
                "id must be specified when using jailer mode".to_string(),
            ));
        }

        if self.jailer_cfg.as_ref().unwrap().gid.is_none() {
            error!("gid must be specified when using jailer mode");
            return Err(MachineError::Validation(
                "gid must be specified when using jailer mode".to_string(),
            ));
        }

        if self.jailer_cfg.as_ref().unwrap().uid.is_none() {
            error!("uid must be specified when using jailer mode");
            return Err(MachineError::Validation(
                "uid must be specified when using jailer mode".to_string(),
            ));
        }

        if self.jailer_cfg.as_ref().unwrap().numa_node.is_none() {
            error!("numa node must be specified when using jailer mode");
            return Err(MachineError::Validation(
                "numa node must be specified when using jailer mode".to_string(),
            ));
        }
        Ok(())
    }

    #[inline]
    pub fn with_socket_path<S: AsRef<OsStr> + ?Sized>(mut self, path: &S) -> Self {
        self.socket_path = Some(path.into());
        self
    }

    #[inline]
    pub fn with_log_fifo<S: AsRef<OsStr> + ?Sized>(mut self, path: &S) -> Self {
        self.log_fifo = Some(path.into());
        self
    }

    #[inline]
    pub fn with_log_path<S: AsRef<OsStr> + ?Sized>(mut self, path: &S) -> Self {
        self.log_path = Some(path.into());
        self
    }

    #[inline]
    pub fn with_log_level(mut self, level: LogLevel) -> Self {
        self.log_level = Some(level);
        self
    }

    #[inline]
    pub fn with_metrics_path<S: AsRef<OsStr> + ?Sized>(mut self, path: &S) -> Self {
        self.metrics_path = Some(path.into());
        self
    }

    #[inline]
    pub fn with_metrics_fifo<S: AsRef<OsStr> + ?Sized>(mut self, path: &S) -> Self {
        self.metrics_fifo = Some(path.into());
        self
    }

    #[inline]
    pub fn with_kernel_image_path<S: AsRef<OsStr> + ?Sized>(mut self, path: &S) -> Self {
        self.kernel_image_path = Some(path.into());
        self
    }

    #[inline]
    pub fn with_kernel_args(mut self, path: &String) -> Self {
        self.kernel_args = Some(path.to_string());
        self
    }

    #[inline]
    pub fn with_initrd_path<S: AsRef<OsStr> + ?Sized>(mut self, path: &S) -> Self {
        self.initrd_path = Some(path.into());
        self
    }

    #[inline]
    pub fn with_drive(mut self, drive: &Drive) -> Self {
        if self.drives.is_none() {
            self.drives = Some(vec![]);
        }
        self.drives.as_mut().unwrap().push(drive.to_owned());
        self
    }

    #[inline]
    pub fn with_drives(mut self, drives: &mut Vec<Drive>) -> Self {
        if self.drives.is_none() {
            self.drives = Some(vec![]);
        }
        self.drives.as_mut().unwrap().append(drives);
        self
    }

    #[inline]
    pub fn with_vsock(mut self, dev: &Vsock) -> Self {
        if self.vsock_devices.is_none() {
            self.vsock_devices = Some(vec![]);
        }
        self.vsock_devices.as_mut().unwrap().push(dev.to_owned());
        self
    }

    #[inline]
    pub fn with_vsocks(mut self, devs: &mut Vec<Vsock>) -> Self {
        if self.vsock_devices.is_none() {
            self.vsock_devices = Some(vec![]);
        }
        self.vsock_devices.as_mut().unwrap().append(devs);
        self
    }

    #[inline]
    pub fn with_machine_config(mut self, cfg: &MachineConfiguration) -> Self {
        self.machine_cfg = Some(cfg.to_owned());
        self
    }

    #[inline]
    pub fn set_disable_validation(mut self, b: bool) -> Self {
        self.disable_validation = b;
        self
    }

    #[inline]
    pub fn with_jailer_config(mut self, cfg: &JailerConfig) -> Self {
        self.jailer_cfg = Some(cfg.to_owned());
        self
    }

    #[inline]
    pub fn with_vmid(mut self, vmid: &String) -> Self {
        self.vmid = Some(vmid.to_owned());
        self
    }

    #[inline]
    pub fn with_net_ns<S: AsRef<OsStr> + ?Sized>(mut self, net_ns: &S) -> Self {
        self.net_ns = Some(net_ns.into());
        self
    }

    #[inline]
    pub fn with_seccomp_level(mut self, level: SeccompLevel) -> Self {
        match level {
            SeccompLevel::Disable => self.seccomp_level = Some(SECCOMP_LEVEL_DISABLE),
            SeccompLevel::Basic => self.seccomp_level = Some(SECCOMP_LEVEL_BASIC),
            SeccompLevel::Advanced => self.seccomp_level = Some(SECCOMP_LEVEL_ADVANCED),
        }
        self
    }

    #[inline]
    pub fn with_mmds_address(mut self, addr: &std::net::Ipv4Addr) -> Self {
        self.mmds_address = Some(addr.to_owned());
        self
    }

    #[inline]
    pub fn with_balloon(mut self, balloon: &Balloon) -> Self {
        self.balloon = Some(balloon.to_owned());
        self
    }

    #[inline]
    pub fn set_log_clear(mut self, b: bool) -> Self {
        self.log_clear = Some(b);
        self
    }

    #[inline]
    pub fn set_metrics_clear(mut self, b: bool) -> Self {
        self.metrics_clear = Some(b);
        self
    }
}

/// Core component of Machine. Serializable and Deserializable.
/// Could be stored as formatted metadata, attached and detached whenever needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineCore {
    // building configuration of the machine
    pub cfg: Config,

    pub socket_path: PathBuf,

    pub firecracker_init_timeout: f64,

    pub firecracker_request_timeout: f64,

    // pid of firecracker process
    pub pid: u32,
}

/// Machine is process handler of rust side
pub struct Machine {
    pub(crate) cfg: Config,

    pub(crate) agent: Agent,

    pub(crate) cmd: Option<tokio::process::Command>,

    /* eliminated in core */
    pub(crate) child_process: Option<tokio::process::Child>,

    /// Whether machine is rebuilt from MachineCore
    pub(crate) rebuilt: bool,

    pub(crate) pid: Option<u32>,

    /// The actual machine config as reported by Firecracker
    /// id est, not the config set by user, which should be a field of `cfg`
    pub(crate) machine_config: MachineConfiguration,
}

unsafe impl Send for Machine {}
unsafe impl Sync for Machine {}

#[derive(thiserror::Error, Debug)]
pub enum MachineError {
    /// Mostly problems related to directories error or unavailable files
    // #[error("Could not set up environment(e.g. file, linking) the machine, reason: {0}")]
    // FileError(String),
    /// Failure when creating file or directory
    #[error("Could not create file or directory, reason: {0}")]
    FileCreation(String),

    /// Failure when the file is missing
    #[error("Could not find file, reason: {0}")]
    FileMissing(String),

    /// Failure when removing the file
    #[error("Could not remove file, reason: {0}")]
    FileRemoving(String),

    /// Failure when accessing file
    #[error("Unable to access file, reason: {0}")]
    FileAccess(String),

    /// Failure when validating the configuration before starting the microVM
    #[error("Invalid configuration for the machine, reason: {0}")]
    Validation(String),

    /// Failure occurred because of missing arguments
    #[error("Missing arguments, reason: {0}")]
    ArgWrong(String),

    /// Related to communication with the socket to configure the microVM which failed
    #[error("Could not put initial configuration for the machine, reason: {0}")]
    Initialize(String),

    /// The process didn't start properly or an error occurred while trying to run it
    #[error("Fail to start or run the machine, reason: {0}")]
    Execute(String),

    /// Failure when cleaning up the machine
    #[error("Could not clean up the machine properly, reason: {0}")]
    Cleaning(String),

    /// An Error occured when communicating with firecracker by Unix Domain Socket
    #[error("Agent could not communicate with firecracker process, reason: {0}")]
    Agent(String),
}

/// functional methods
impl Machine {
    /// new initializes a new Machine instance and performs validation of the
    /// provided Config.
    pub fn new(cfg: Config) -> Result<Machine, MachineError> {
        /* Validate Config */
        cfg.validate()?;

        /* Validate network */
        cfg.validate_network()?;

        // Re-write cfg
        let (cfg, cmd) = Machine::jail(cfg)?;
        debug!(target: "Machine Config", "{}", format!("{:#?}", cfg));

        let agent_init_timeout = std::env::var(FIRECRACKER_INIT_TIMEOUT_ENV);
        let agent_init_timeout = match agent_init_timeout {
            Ok(t) => t.as_str().parse().map_err(|e| {
                error!(target: "Machine::new", "non-number value for agent init timeout");
                MachineError::ArgWrong(format!(
                    "non-number value for agent init timeout {}: {}",
                    t, e
                ))
            })?,
            Err(_) => DEFAULT_FIRECRACKER_INIT_TIMEOUT_SECONDS,
        };
        let agent_request_timeout = std::env::var(FIRECRACKER_REQUEST_TIMEOUT_ENV);
        let agent_request_timeout = match agent_request_timeout {
            Ok(t) => t.as_str().parse().map_err(|e| {
                error!(target: "Machine::new", "non-number value for agent request timeout");
                MachineError::ArgWrong(format!(
                    "non-number value for agent request timeout {}: {}",
                    t, e
                ))
            })?,
            Err(_) => DEFAULT_FIRECRACKER_REQUEST_TIMEOUT_SECONDS,
        };

        let agent = Agent::new(
            cfg.socket_path.as_ref().ok_or(MachineError::Initialize(
                "no socket_path provided in the config".to_string(),
            ))?,
            agent_request_timeout,
            agent_init_timeout,
        );
        info!(target: "Machine::new", "machine agent created monitoring socket at {:#?}", cfg.socket_path.as_ref().unwrap());

        let machine_config = cfg
            .machine_cfg
            .as_ref()
            .ok_or(MachineError::Initialize(
                "no machine_config provided in the config".to_string(),
            ))?
            .to_owned();

        let machine = Machine {
            agent,
            machine_config,
            cfg,
            cmd: Some(cmd),
            child_process: None,
            rebuilt: false,
            pid: None,
        };
        Ok(machine)
    }

    /// Rebuild Machine from raw metadata (MachineCore)
    pub fn rebuild(core: MachineCore) -> Result<Machine, MachineError> {
        let agent = Agent::new(
            &core.socket_path,
            core.firecracker_request_timeout,
            core.firecracker_init_timeout,
        );
        let machine = Self {
            cfg: core.cfg,
            agent,
            cmd: None,
            child_process: None,
            rebuilt: true,
            pid: Some(core.pid),
            machine_config: MachineConfiguration::default(),
        };

        Ok(machine)
    }

    /// Dump the Machine to MachineCore
    pub fn dump_into_core(&self) -> Result<MachineCore, MachineError> {
        let core = MachineCore {
            cfg: self.cfg.to_owned(),
            socket_path: self.agent.socket_path.to_owned(),
            firecracker_init_timeout: self.agent.firecracker_init_timeout,
            firecracker_request_timeout: self.agent.firecracker_request_timeout,
            pid: self.pid()?,
        };
        Ok(core)
    }

    /// Start actually start a Firecracker microVM.
    pub async fn start(&mut self) -> Result<(), MachineError> {
        debug!(target: "Machine::start", "called Machine::start");

        // 1. start firecracker process
        // added socket clear
        self.start_vmm().await?;

        // 2. create log files (and link files, when jailing)
        // added clear log fifo
        self.create_log_fifo_or_file()?;

        // 3. create metrics files (and link files, when jailing)
        // added clear metrics fifo
        self.create_metrics_fifo_or_file()?;

        // 4. redirect io, copy log_fifo to specified position
        // self.capture_fifo_to_file().await?;

        // 5. link files
        self.link_files().await?;

        // 6. bootstrap logging
        self.setup_logging().await?;

        // 7. bootstrap metrics
        self.setup_metrics().await?;

        // 8. put machine configuration
        self.create_machine().await?;

        // 9. put boot source
        self.create_boot_source(
            &self.cfg.kernel_image_path.as_ref().unwrap(),
            &self.cfg.initrd_path,
            &self.cfg.kernel_args,
        )
        .await?;

        // 10. attach drives
        self.attach_drives().await?;

        // 11. create network interfaces
        self.create_network_interfaces().await?;

        // 12. add virtio socks
        self.add_vsocks().await?;

        // 13. optional set mmds config
        self.set_mmds_config().await?;

        // 14. optional put mmds metadata
        self.set_metadata().await?;

        // 15. optional create balloon
        self.create_balloon().await?;

        // 16. send instance start action
        let start_res = self.start_instance().await;
        if let Err(e) = start_res {
            error!(target: "Machine::start", "fail when sending instance start action: {}", e);
            // do cleaning up to clear things left by this fail starting
            self.do_clean_up().await.map_err(|e| {
                error!(
                    target: "Machine::start",
                    "start failed when do cleaning after instance starting failed: {}",
                    e
                );
                MachineError::Cleaning(format!(
                    "start failed when do cleaning after instance starting failed: {}",
                    e.to_string()
                ))
            })?;
            return Err(MachineError::Execute(format!(
                "Machine::start failed due to {}",
                e
            )));
        }
        Ok(())
    }

    /// wait will wait until the firecracker process has finished,
    /// or has been forced to terminate.
    pub async fn wait(&mut self) -> Result<(), MachineError> {
        debug!(target: "Machine::wait", "called Machine::wait");
        if !self.rebuilt && (self.cmd.is_none() || self.child_process.is_none()) {
            error!(target: "Machine::wait", "cannot wait before machine starts");
            return Err(MachineError::Execute(
                "cannot wait before machine starts".to_string(),
            ));
        }

        if self.rebuilt {
            // if Machine is rebuilt from MachineCore, no child_process provided, only pid
            let output = nix::sys::wait::waitpid(
                nix::unistd::Pid::from_raw(self.pid()? as i32),
                Some(nix::sys::wait::WaitPidFlag::WUNTRACED | nix::sys::wait::WaitPidFlag::WEXITED),
            );
            match output {
                Err(e) => warn!(target: "Machine::wait", "firecracker exited: {}", e),
                Ok(status) => match status {
                    nix::sys::wait::WaitStatus::Exited(_pid, code) => {
                        info!(target: "Machine::wait", "firecracker exited successfully: {}", code)
                    }
                    _ => warn!(target: "Machine::wait", "firecracker exited abnormally"),
                },
            }
        } else {
            let output = self.child_process.as_mut().unwrap().wait().await;
            match output {
                Err(e) => warn!(target: "Machine::wait", "firecracker exited: {}", e),
                Ok(status) => {
                    info!(target: "Machine::wait", "firecracker exited successfully: {}", status)
                }
            }
        }
        self.do_clean_up().await.map_err(|e| {
            error!(target: "Machine::wait", "fail to do cleaning up: {}", e);
            MachineError::Cleaning(format!("fail to do cleaning up: {}", e))
        })?;

        info!(target: "Machine::wait", "machine {} exited successfully", self.cfg.vmid.as_ref().unwrap());

        Ok(())
    }

    /// shutdown requests a clean shutdown of the VM by sending CtrlAltDelete on the virtual keyboard
    pub async fn shutdown(&self) -> Result<(), MachineError> {
        debug!(target: "Machine::shutdown", "called Machine::shutdown");
        self.send_ctrl_alt_del().await
    }

    /// pause pauses the microVM
    pub async fn pause(&self) -> Result<(), MachineError> {
        debug!(target: "Machine::pause", "called Machine::pause");
        self.agent.patch_vm(&VM_STATE_PAUSED).await.map_err(|e| {
            error!(target: "Machine::pause", "sending failure: {}", e);
            MachineError::Execute(e.to_string())
        })?;
        info!(target: "Machine::pause", "Machine {} paused", self.cfg.vmid.as_ref().unwrap());
        Ok(())
    }

    /// resume resumes the microVM from pausing
    pub async fn resume(&self) -> Result<(), MachineError> {
        debug!(target: "Machine::resume", "called Machine::resume");
        self.agent.patch_vm(&VM_STATE_RESUMED).await.map_err(|e| {
            error!(target: "Machine::resume", "sending failure: {}", e);
            MachineError::Execute(e.to_string())
        })?;
        info!(target: "Machine::resume", "Machine {} resumed", self.cfg.vmid.as_ref().unwrap());
        Ok(())
    }

    /// stop_vmm stops the current VMM by sending a SIGTERM
    pub async fn stop_vmm(&mut self) -> Result<(), MachineError> {
        debug!(target: "Machine::stop_vmm", "sending sigterm to firecracker");

        // sending a SIGTERM
        if self.cmd.is_some() && self.child_process.is_some() {
            let pid = self
                .child_process
                .as_ref()
                .unwrap()
                .id()
                .ok_or(MachineError::Execute(
                    "stop_vmm: no pid found, maybe VMM already stopped".to_string(),
                ))?;
            nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid as i32),
                nix::sys::signal::SIGTERM,
            )
            .map_err(|e| {
                error!(
                    target: "Machine::stop_vmm",
                    "fail to send SIGTERM to firecracker process {}, reason: {}",
                    pid, e
                );
                MachineError::Execute(format!(
                    "fail to send SIGTERM to firecracker process {}, reason: {}",
                    pid, e
                ))
            })?
        } else {
            info!(target: "Machine::stop_vmm", "no firecracker process running, not sending SIGTERM");
        }

        self.do_clean_up().await?;

        Ok(())
    }

    /// stop_vmm_force stops the current VMM by sending a SIGKILL
    pub async fn stop_vmm_force(&mut self) -> Result<(), MachineError> {
        debug!(target: "Machine::stop_vmm_force", "sending sigkill to firecracker");

        // sending a SIGKILL
        if self.cmd.is_some() && self.child_process.is_some() {
            let pid = self.pid;
            self.child_process
                .as_mut()
                .unwrap()
                .kill()
                .await
                .map_err(|e| {
                    error!(target: "Machine::stop_vmm_force", "vmm process already finished!");
                    MachineError::Execute(format!(
                        "firecracker process already finished, pid {:?}: {}",
                        pid, e
                    ))
                })?;
        } else {
            info!(target: "Machine::stop_vmm_force", "stop_vmm_force: no firecracker process running, not sending SIGKILL");
        }
        
        self.do_clean_up().await?;

        Ok(())
    }
}

/// private methods
impl Machine {
    /// wait_for_socket waits for the given file to exist
    async fn wait_for_socket(&self, timeout_in_secs: f64) -> Result<(), MachineError> {
        if self.cfg.socket_path.is_none() {
            return Err(MachineError::ArgWrong(
                "socket path not provided in the configuration".to_string(),
            ));
        }
        tokio::time::timeout(
            tokio::time::Duration::from_secs_f64(timeout_in_secs),
            async move {
                while let Err(_) = tokio::fs::metadata(self.cfg.socket_path.as_ref().unwrap()).await
                {
                }
                debug!("firecracker created socket at the given path");
            },
        )
        .await
        .map_err(|_| {
            MachineError::Initialize(format!(
                "firecracker fail to create socket at the given path after {} seconds",
                timeout_in_secs
            ))
        })?;

        Ok(())
    }

    /// create_network_interface creates network interface
    async fn create_network_interface(&self, iface: &NetworkInterface) -> Result<(), MachineError> {
        self.agent.put_guest_network_interface_by_id(iface).await.map_err(|e| {
            error!(target: "Machine::create_network_interface", "PutGuestNetworkInterfaceByID: {}", e);
            MachineError::Agent(format!("PutGuestNetworkInterfaceByID: {}", e.to_string()))
        })?;

        debug!(target: "Machine::create_network_interface", "PutGuestNetworkInterfaceByID successful");
        Ok(())
    }

    /// attach_drive attaches a secondary block device.
    async fn attach_drive(&self, dev: &Drive) -> Result<(), MachineError> {
        let host_path = dev.get_path_on_host();
        info!(
            "Attaching drive {}, slot {}, root {}",
            host_path.display(),
            dev.get_drive_id(),
            dev.is_root_device()
        );
        self.agent.put_guest_drive_by_id(dev).await.map_err(|e| {
            error!(
                "Attach drive failed: {}: {}",
                host_path.display(),
                e.to_string()
            );
            MachineError::Agent(format!(
                "Attach drive failed: {}: {}",
                host_path.display(),
                e.to_string()
            ))
        })?;

        info!("Attached drive {}", host_path.display());
        Ok(())
    }

    /// add_vsock adds a vsock to the instance
    async fn add_vsock(&self, vsock: &Vsock) -> Result<(), MachineError> {
        self.agent.put_guest_vsock(vsock).await.map_err(|e| {
            MachineError::Agent(format!("PutGuestVsock returned: {}", e.to_string()))
        })?;
        info!("attch vsock {} successful", vsock.uds_path.display());
        Ok(())
    }

    async fn do_clean_up(&mut self) -> Result<(), MachineError> {
        debug!(target: "Machine::do_clean_up", "called Machine::do_clean_up");
        
        // Without jailer
        if let Some(true) = self.cfg.log_clear {
            if let Err(e) = self.clear_file(&self.cfg.log_fifo).await {
                warn!(target: "Machine::do_clean_up", "when removing log_fifo {}: {}", self.cfg.log_fifo.as_ref().unwrap().display(), e);
            }
            if let Err(e) = self.clear_file(&self.cfg.log_path).await {
                warn!(target: "Machine::do_clean_up", "when removing log_path {}: {}", self.cfg.log_path.as_ref().unwrap().display(), e);
            }
            if self.cfg.enable_jailer {
                if let Err(e) = self.clear_file(&self.cfg.jailer_cfg.as_ref().unwrap().log_link_dest).await {
                    warn!(target: "Machine::do_clean_up", "when removing log_fifo {}: {}", self.cfg.log_fifo.as_ref().unwrap().display(), e);
                }
                if let Err(e) = self.clear_file(&self.cfg.jailer_cfg.as_ref().unwrap().log_link_src).await {
                    warn!(target: "Machine::do_clean_up", "when removing log_fifo {}: {}", self.cfg.log_fifo.as_ref().unwrap().display(), e);
                }
            }
        }

        if let Some(true) = self.cfg.metrics_clear {
            if let Err(e) = self.clear_file(&self.cfg.metrics_fifo).await {
                warn!(target: "Machine::do_clean_up", "when removing metrics_fifo {}: {}", self.cfg.metrics_fifo.as_ref().unwrap().display(), e);
            }
            if let Err(e) = self.clear_file(&self.cfg.metrics_path).await {
                warn!(target: "Machine::do_clean_up", "when removing metrics_path {}: {}", self.cfg.metrics_path.as_ref().unwrap().display(), e);
            }
            if self.cfg.enable_jailer {
                if let Err(e) = self.clear_file(&self.cfg.jailer_cfg.as_ref().unwrap().metrics_link_dest).await {
                    warn!(target: "Machine::do_clean_up", "when removing metrics_fifo {}: {}", self.cfg.metrics_fifo.as_ref().unwrap().display(), e);
                }
                if let Err(e) = self.clear_file(&self.cfg.jailer_cfg.as_ref().unwrap().metrics_link_src).await {
                    warn!(target: "Machine::do_clean_up", "when removing metrics_fifo {}: {}", self.cfg.metrics_fifo.as_ref().unwrap().display(), e);
                }
            }
        }

        if let Err(e) = self.clear_file(&self.cfg.socket_path).await {
            warn!(target: "Machine::do_clean_up", "when removing socket_path {}: {}", self.cfg.socket_path.as_ref().unwrap().display(), e);
        }

        if let Some(true) = self.cfg.network_clear {
            if let Err(e) = self.clear_network().await {
                warn!(target: "Machine::do_clean_up", "when clearing network: {}", e);
            }
        }

        info!(target: "Machine::do_clean_up", "Machine {} cleaned", self.cfg.vmid.as_ref().unwrap());
        Ok(())
    }

    /// called by shutdown, which is called by user to perform graceful shutdown
    async fn send_ctrl_alt_del(&self) -> Result<(), MachineError> {
        debug!(target: "Machine::send_ctrl_alt_del", "called Machine::send_ctrl_alt_del");
        self.agent
            .create_sync_action(&InstanceActionInfo::send_ctrl_alt_del())
            .await
            .map_err(|e| {
                error!(target: "Machine::send_ctrl_alt_del", "sending failure: {}", e);
                MachineError::Execute(e.to_string())
            })?;
        Ok(())
    }

    /// start_instance: send InstanceActionInfo::instance_start() to firecracker process.
    /// Should be called only by Machine::start, after start_vmm has returned successfully.
    async fn start_instance(&self) -> Result<(), MachineError> {
        debug!(target: "Machine::start_instance", "called Machine::start_instance");
        self.agent
            .create_sync_action(&InstanceActionInfo::instance_start())
            .await
            .map_err(|e| {
                error!(target: "Machine::start_instance", "sending failure: {}", e);
                MachineError::Execute(e.to_string())
            })?;
        info!(target: "Machine::start_instance", "instance start sent");
        Ok(())
    }

    /// clear_file: clear file set by firecracker
    async fn clear_file(&self, path: &Option<PathBuf>) -> Result<(), MachineError> {
        debug!(target: "Machine::clear_file", "called Machine::clear_file");
        if path.is_none() {
            warn!(target: "Machine::clear_file", "no need of clearing up, found None");
            return Ok(());
        }
        let path = path.as_ref().unwrap();
        info!(target: "Machine::clear_file", "clearing {}", path.display());

        std::fs::remove_file(path).map_err(|e| {
            MachineError::Cleaning(format!(
                "fail to remove the file at {}: {}",
                path.display(),
                e.to_string()
            ))
        })?;
        if let Ok(_) = std::fs::metadata(path) {
            return Err(MachineError::Cleaning(format!(
                "fail to remove the file at {}, maybe a dir, non-exist file or permission deny",
                path.display()
            )));
        }
        Ok(())
    }

    /// clear_network: clear network settings
    async fn clear_network(&self) -> Result<(), MachineError> {
        Ok(())
    }

    /// linking files: link files to jailer directory if jailer config exists
    async fn link_files(&mut self) -> Result<(), MachineError> {
        if !self.cfg.enable_jailer {
            warn!(target: "Machine::link_files", "jailer config was not set for use");
            return Ok(());
        }
        let jcfg = self.cfg.jailer_cfg.as_ref().unwrap();

        // assemble target path
        let chroot_base_dir: PathBuf = jcfg
            .chroot_base_dir
            .to_owned()
            .unwrap_or(DEFAULT_JAILER_PATH.into());
        let exec_file_path: PathBuf = jcfg
            .exec_file
            .as_ref()
            .unwrap()
            .as_path()
            .file_name()
            .ok_or(MachineError::ArgWrong(format!(
                "malformed firecracker exec file name"
            )))?
            .into();
        let id_string: PathBuf = jcfg.id.as_ref().unwrap().into();
        let rootfs: PathBuf = [
            chroot_base_dir,
            exec_file_path,
            id_string,
            ROOTFS_FOLDER_NAME.into(),
        ]
        .iter()
        .collect();

        // hard link kernel image to root folder
        let kernel_image_name: PathBuf = self
            .cfg
            .kernel_image_path
            .as_ref()
            .unwrap()
            .as_path()
            .file_name()
            .ok_or(MachineError::ArgWrong(format!(
                "malformed kernel image path"
            )))?
            .into();
        let kernel_image_name_full: PathBuf = [&rootfs, &kernel_image_name].iter().collect();
        std::fs::hard_link(
            self.cfg.kernel_image_path.as_ref().unwrap(),
            &kernel_image_name_full,
        )
        .map_err(|e| {
            error!("fail to copy kernel image to root fs: {}", e.to_string());
            MachineError::FileAccess(format!(
                "fail to copy kernel image to root fs: {}",
                e.to_string()
            ))
        })?;
        // reset the kernel image path in configuration
        self.cfg.kernel_image_path = Some(kernel_image_name);

        // hard link initrd drive to root folder (if present)
        if self.cfg.initrd_path.is_some() {
            let initrd_file_name: PathBuf = self
                .cfg
                .initrd_path
                .as_ref()
                .unwrap()
                .as_path()
                .file_name()
                .ok_or(MachineError::ArgWrong(format!("malformed initrd path")))?
                .into();
            let initrd_file_name_full: PathBuf = [&rootfs, &initrd_file_name].iter().collect();
            std::fs::hard_link(
                self.cfg.initrd_path.as_ref().unwrap(),
                initrd_file_name_full,
            )
            .map_err(|e| {
                error!("fail to copy initrd device to root fs: {}", e.to_string());
                MachineError::FileAccess(format!(
                    "fail to copy initrd device to root fs: {}",
                    e.to_string()
                ))
            })?;
            self.cfg.initrd_path = Some(initrd_file_name);
        }

        // hard link all drives to root folder (if present)
        for drive in self.cfg.drives.as_mut().unwrap() {
            let host_path = &drive.get_path_on_host();
            let drive_file_name: PathBuf = host_path
                .as_path()
                .file_name()
                .ok_or(MachineError::ArgWrong(
                    "malformed drive file name".to_string(),
                ))?
                .into();
            let drive_file_name_full: PathBuf = [&rootfs, &drive_file_name].iter().collect();
            std::fs::hard_link(&host_path, &drive_file_name_full).map_err(|e| {
                error!("fail to copy drives to root fs: {}", e.to_string());
                MachineError::FileAccess(format!(
                    "fail to copy drives to root fs: {}",
                    e.to_string()
                ))
            })?;

            // reset the path_on_host field to new one
            drive.set_drive_path(drive_file_name);
        }

        // hard link log fifos to root folder (if present)
        if self.cfg.log_fifo.is_some() {
            let file_name: PathBuf = self
                .cfg
                .log_fifo
                .as_ref()
                .unwrap()
                .as_path()
                .file_name()
                .ok_or(MachineError::ArgWrong("malformed fifo path".to_string()))?
                .into();
            let file_name_full: PathBuf = [&rootfs, &file_name].iter().collect();
            std::fs::hard_link(self.cfg.log_fifo.as_ref().unwrap(), &file_name_full).map_err(|e| {
                error!(target: "Machine::link_files", "fail to copy fifo file to root fs: {}", e.to_string());
                MachineError::FileAccess(format!(
                    "fail to copy fifo file to root fs: {}",
                    e.to_string()
                ))
            })?;

            // chown
            nix::unistd::chown(
                &file_name_full,
                Some(nix::unistd::Uid::from_raw(
                    *self.cfg.jailer_cfg.as_ref().unwrap().uid.as_ref().unwrap(),
                )),
                Some(nix::unistd::Gid::from_raw(
                    *self.cfg.jailer_cfg.as_ref().unwrap().gid.as_ref().unwrap(),
                )),
            )
            .map_err(|e| {
                error!(target: "Machine::link_files", "fail to chown: {}", e.to_string());
                MachineError::FileAccess(format!("fail to chown: {}", e.to_string()))
            })?;

            // reset fifo path
            self.cfg.jailer_cfg.as_mut().unwrap().log_link_src = self.cfg.log_fifo.take();
            self.cfg.jailer_cfg.as_mut().unwrap().log_link_dest = Some(file_name_full);
            self.cfg.log_fifo = Some(file_name);
        }

        // hard link metrics fifo to root folder (if present)
        if self.cfg.metrics_fifo.is_some() {
            let file_name: PathBuf = self
                .cfg
                .metrics_fifo
                .as_ref()
                .unwrap()
                .as_path()
                .file_name()
                .ok_or(MachineError::ArgWrong("malformed fifo path".to_string()))?
                .into();
            let file_name_full: PathBuf = [&rootfs, &file_name].iter().collect();
            std::fs::hard_link(self.cfg.metrics_fifo.as_ref().unwrap(), &file_name_full).map_err(|e| {
                error!(target: "Machine::link_files", "fail to copy fifo file to root fs: {}", e.to_string());
                MachineError::FileAccess(format!(
                    "fail to copy fifo file to root fs: {}",
                    e.to_string()
                ))
            })?;
            // chown
            nix::unistd::chown(
                &file_name_full,
                Some(nix::unistd::Uid::from_raw(
                    *self.cfg.jailer_cfg.as_ref().unwrap().uid.as_ref().unwrap(),
                )),
                Some(nix::unistd::Gid::from_raw(
                    *self.cfg.jailer_cfg.as_ref().unwrap().gid.as_ref().unwrap(),
                )),
            )
            .map_err(|e| {
                error!(target: "Machine::link_files", "fail to chown: {}", e.to_string());
                MachineError::FileAccess(format!("fail to chown: {}", e.to_string()))
            })?;

            // reset fifo path
            self.cfg.jailer_cfg.as_mut().unwrap().metrics_link_src = self.cfg.metrics_fifo.take();
            self.cfg.jailer_cfg.as_mut().unwrap().metrics_link_dest = Some(file_name_full);
            self.cfg.metrics_fifo = Some(file_name);
        }
        Ok(())
    }

    /// jail will set up proper handlers and remove configuration validation due to
    /// stating of files
    fn jail(cfg: Config) -> Result<(Config, Command), MachineError> {
        if !cfg.enable_jailer {
            let cmd = VMMCommandBuilder::default()
                .with_socket_path(cfg.socket_path.as_ref().unwrap())
                .with_args(vec![
                    "--seccomp-level".to_string(),
                    cfg.seccomp_level
                        .unwrap_or(SECCOMP_LEVEL_DISABLE)
                        .to_string(),
                    "--id".to_string(),
                    cfg.vmid.as_ref().unwrap().to_string(),
                ]).build().into();
            return Ok((cfg, cmd));
        }

        let mut cfg = cfg.clone();

        if cfg.jailer_cfg.is_none() {
            return Err(MachineError::Initialize(
                "jailer config was not set for use".to_string(),
            ));
        }

        // assemble machine socket path
        let machine_socket_path: PathBuf;
        if let Some(socket_path) = &cfg.socket_path {
            machine_socket_path = socket_path.to_path_buf();
        } else {
            return Err(MachineError::ArgWrong("No socket_path provided".to_string()))
        }

        let jailer_workspace_dir: PathBuf;
        let jailer_cfg = cfg.jailer_cfg.as_ref().unwrap();

        let exec_file_name: PathBuf = jailer_cfg
            .exec_file
            .as_ref()
            .unwrap()
            .file_name()
            .ok_or(MachineError::ArgWrong(
                "malformed firecracker exec file name".to_string(),
            ))?
            .into();
        let id_string: PathBuf = jailer_cfg.id.as_ref().unwrap().into();

        if let Some(chroot_base_dir) = &jailer_cfg.chroot_base_dir {
            jailer_workspace_dir = [
                chroot_base_dir.to_owned(),
                exec_file_name,
                id_string,
                ROOTFS_FOLDER_NAME.into(),
            ]
            .iter()
            .collect();
        } else {
            jailer_workspace_dir = [
                PathBuf::from(DEFAULT_JAILER_PATH),
                exec_file_name,
                id_string,
                ROOTFS_FOLDER_NAME.into(),
            ]
            .iter()
            .collect();
        }

        // reset the socket_path
        cfg.socket_path = Some(jailer_workspace_dir.join(&machine_socket_path));

        // open stdio
        let mut stdout = std::process::Stdio::inherit();
        if jailer_cfg.stdout.is_some() {
            stdout = jailer_cfg.stdout.as_ref().unwrap().open_io()?;
        }

        let mut stderr = std::process::Stdio::inherit();
        if jailer_cfg.stderr.is_some() {
            stderr = jailer_cfg.stderr.as_ref().unwrap().open_io()?;
        }

        let mut stdin = std::process::Stdio::inherit();
        if jailer_cfg.stdin.is_some() {
            stdin = jailer_cfg.stdin.as_ref().unwrap().open_io()?;
        }

        let mut builder = JailerCommandBuilder::new()
            .with_id(jailer_cfg.id.as_ref().unwrap())
            .with_uid(jailer_cfg.uid.as_ref().unwrap())
            .with_gid(jailer_cfg.gid.as_ref().unwrap())
            .with_numa_node(jailer_cfg.numa_node.as_ref().unwrap())
            .with_exec_file(jailer_cfg.exec_file.as_ref().unwrap())
            .with_chroot_base_dir(
                jailer_cfg
                    .chroot_base_dir
                    .to_owned()
                    .unwrap_or(DEFAULT_JAILER_PATH.into()),
            )
            .with_daemonize(jailer_cfg.daemonize.as_ref().unwrap())
            .with_firecracker_args(vec![
                "--api-sock".to_string(),
                machine_socket_path.to_string_lossy().to_string(),
            ])
            .with_stdout(stdout)
            .with_stderr(stderr)
            .with_stdin(stdin);

        if let Some(jailer_binary) = &jailer_cfg.jailer_binary {
            builder = builder.with_bin(jailer_binary);
        }

        if let Some(net_ns) = &cfg.net_ns {
            builder = builder.with_net_ns(net_ns);
        }

        let cmd = builder.build().into();

        Ok((cfg, cmd))
    }
}

impl Default for Machine {
    /// default returns a blanck machine which should be configured
    fn default() -> Self {
        Machine {
            cfg: Config::default(),
            agent: Agent::blank(),
            cmd: None,
            child_process: None,
            rebuilt: false,
            pid: None,
            machine_config: MachineConfiguration::default(),
        }
    }
}

/// util methods
impl Machine {
    pub fn get_log_file(&self) -> Option<PathBuf> {
        self.cfg.log_fifo.to_owned()
    }

    /// Set the boot command mannually
    pub fn set_command(&mut self, cmd: tokio::process::Command) {
        self.cmd = Some(cmd);
    }

    /// PID returns the machine's running process PID or an error if not running
    pub fn pid(&self) -> Result<u32, MachineError> {
        if self.cmd.is_some() && self.child_process.is_some() {
            self.child_process
                .as_ref()
                .unwrap()
                .id()
                .ok_or(MachineError::Execute("process terminated".to_string()))
        } else if self.pid.is_some() {
            self.pid
                .ok_or(MachineError::Execute("Malformed Machine".to_string()))
        } else {
            return Err(MachineError::Execute("process terminated".to_string()));
        }
    }

    /// Get boot Config
    pub fn get_config(&self) -> Config {
        self.cfg.to_owned()
    }
}

/// method that should be called in start
impl Machine {
    /// start_vmm starts the firecracker vmm process.
    pub(super) async fn start_vmm(&mut self) -> Result<(), MachineError> {
        debug!(target: "Machine::start_vmm", "called Machine::start_vmm");
        if self.cfg.socket_path.is_none() {
            error!(target: "Machine::start_vmm", "no socket path provided");
            return Err(MachineError::ArgWrong(
                "start_vmm: no socket path provided".to_string(),
            ));
        }
        info!(
            target: "Machine::start_vmm",
            "called start_vmm, setting up a VMM with socket path: {}",
            self.cfg.socket_path.as_ref().unwrap().display()
        );

        if self.cmd.is_none() {
            error!(target: "Machine::start_vmm", "no command provided");
            return Err(MachineError::Execute("no command provided".to_string()));
        }
        debug!(target: "Machine::start_vmm", "starting command:\n{:#?}", self.cmd.as_ref().unwrap());

        let start_result;

        if self.cfg.net_ns.is_some() && self.cfg.jailer_cfg.is_none() {
            // If the VM needs to be started in a netns but no jailer netns was configured,
            // start the vmm child process in the netns directly here.
            start_result = self.cmd.as_mut().unwrap().spawn();
        } else {
            // Else, just start the process normally as it's either not in a netns or will
            // be placed in one by the jailer process instead.
            start_result = self.cmd.as_mut().unwrap().spawn();
        }
        info!(target: "Machine::start_vmm", "command called");

        if let Err(e) = start_result {
            error!(target: "Machine::start_vmm", "Failed to start vmm: {}", e.to_string());
            return Err(MachineError::Execute(format!(
                "failed to start vmm: {}",
                e.to_string()
            )));
        } else {
            let process = start_result.unwrap();
            let pid = process.id();
            self.child_process = Some(process);
            self.pid = pid;
        }
        debug!(
            target: "Machine::start_vmm",
            "VMM started socket path is: {}",
            self.cfg.socket_path.as_ref().unwrap().display()
        );

        self.wait_for_socket(self.agent.firecracker_init_timeout)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                error!(target: "Machine::start_vmm", "firecracker did not create API socket {}", self.cfg.socket_path.as_ref().unwrap().display());
                MachineError::Initialize(format!(
                    "firecracker did not create API socket {}: {}",
                    self.cfg.socket_path.as_ref().unwrap().display(),
                    msg
                ))
            })?;

        debug!(target: "Machine::start_vmm", "exiting start_vmm");
        Ok(())
    }

    /// called by CreateLogFilesHandler
    pub(super) fn create_log_fifo_or_file(&mut self) -> Result<(), MachineError> {
        debug!(target: "Machine::create_log_fifo_or_file", "called create_log_fifo_or_file");
        if let Some(fifo) = &self.cfg.log_fifo {
            unistd::mkfifo(fifo, Mode::S_IRUSR | Mode::S_IWUSR).map_err(|e| {
                error!(target: "Machine::create_log_fifo_or_file", "fail to create fifo at {}: {}", fifo.display(), e);
                MachineError::FileCreation(format!(
                    "cannot make fifo at {}: {}",
                    fifo.display(),
                    e.to_string()
                ))
            })?;

            Ok(())
        } else if let Some(path) = &self.cfg.log_path {
            let raw_fd = fcntl::open(
                path,
                fcntl::OFlag::O_RDWR | fcntl::OFlag::O_CREAT | fcntl::OFlag::O_APPEND,
                Mode::S_IRUSR | Mode::S_IWUSR,
            )
            .map_err(|e| {
                error!(target: "Machine::create_log_fifo_or_file", "fail to create file: {}", e);
                MachineError::FileCreation(format!("cannot make file: {}", e.to_string()))
            })?;
            unistd::close(raw_fd).map_err(|e| {
                debug!(target: "Machine::create_log_fifo_or_file", "fail to close file at {}: {}", path.display(), e.to_string());
                MachineError::FileCreation(format!(
                    "fail to close file at {}: {}",
                    path.display(),
                    e.to_string()
                ))
            })?;
            Ok(())
        } else {
            info!(target: "Machine::create_log_fifo_or_file", "no log file path provided, just return");
            Ok(())
        }
    }

    /// called by CreateLogFilesHandler
    pub(super) fn create_metrics_fifo_or_file(&mut self) -> Result<(), MachineError> {
        if let Some(fifo) = &self.cfg.metrics_fifo {
            unistd::mkfifo(fifo, Mode::S_IRUSR | Mode::S_IWUSR).map_err(|e| {
                MachineError::FileCreation(format!(
                    "cannot make fifo at {}: {}",
                    fifo.display(),
                    e.to_string()
                ))
            })?;

            Ok(())
        } else if let Some(path) = &self.cfg.metrics_path {
            let raw_fd = fcntl::open(
                path,
                fcntl::OFlag::O_RDWR | fcntl::OFlag::O_CREAT | fcntl::OFlag::O_APPEND,
                Mode::S_IRUSR | Mode::S_IWUSR,
            )
            .map_err(|e| {
                MachineError::FileCreation(format!("cannot make file: {}", e.to_string()))
            })?;
            unistd::close(raw_fd).map_err(|e| {
                MachineError::FileCreation(format!(
                    "fail to close file at {}: {}",
                    path.display(),
                    e.to_string()
                ))
            })?;
            Ok(())
        } else {
            info!(target: "Machine::create_metrics_fifo_or_file", "no metrics file path provided, just return");
            Ok(())
        }
    }

    pub(super) async fn setup_logging(&self) -> Result<(), MachineError> {
        let path: &PathBuf;
        if self.cfg.log_fifo.is_some() {
            path = self.cfg.log_fifo.as_ref().unwrap();
        } else if self.cfg.log_path.is_some() {
            path = self.cfg.log_path.as_ref().unwrap();
        } else {
            info!(target: "Machine::setup_logging", "VMM logging disabled");
            return Ok(());
        }

        let mut l = Logger::default()
            .with_log_path(path)
            .set_show_level(true)
            .set_show_origin(false);
        if self.cfg.log_level.is_some() {
            l = l.with_log_level(self.cfg.log_level.as_ref().unwrap());
        }

        self.agent.put_logger(&l).await.map_err(|e| {
            error!(
                "Fail to configured VMM logging to {}: {}",
                path.display(),
                e.to_string()
            );
            MachineError::Initialize(format!(
                "Fail to configured VMM logging to {}: {}",
                path.display(),
                e.to_string()
            ))
        })?;
        debug!("Configured VMM logging to {}", path.display());
        Ok(())
    }

    pub(super) async fn setup_metrics(&self) -> Result<(), MachineError> {
        let path: &PathBuf;
        if self.cfg.metrics_fifo.is_some() {
            path = self.cfg.metrics_fifo.as_ref().unwrap();
        } else if self.cfg.metrics_path.is_some() {
            path = self.cfg.metrics_path.as_ref().unwrap();
        } else {
            info!(target: "Machine::setup_metrics", "VMM metrics disabled");
            return Ok(());
        }
        let metrics = Metrics::default().with_metrics_path(path);
        self.agent.put_metrics(&metrics).await.map_err(|e| {
            debug!("Configured VMM metrics to {}", path.display());
            MachineError::Agent(format!("Setup metrics with agent error: {}", e.to_string()))
        })?;
        Ok(())
    }

    /// create_machine put the machine configuration to firecracker
    /// and refresh(by get from firecracker) the machine configuration stored in `self`
    pub(super) async fn create_machine(&mut self) -> Result<(), MachineError> {
        debug!(target: "Machine::create_machine", "called Machine::create_machine");
        if self.cfg.machine_cfg.is_none() {
            // one must provide machine config
            error!(target: "Machine::create_machine", "no machine config provided");
            return Err(MachineError::Execute(
                "no machine config provided".to_string(),
            ));
        }
        self.agent
            .put_machine_configuration(self.cfg.machine_cfg.as_ref().unwrap())
            .await
            .map_err(|e| {
                error!(target: "Machine::create_machine", "fail to put machine configuration");
                MachineError::Initialize(format!(
                    "PutMachineConfiguration returned {}",
                    e.to_string()
                ))
            })?;
        debug!(target: "Machine::create_machine", "PutMachineConfiguration returned");
        self.refresh_machine_configuration().await?;
        debug!(target: "Machine::create_machine", "exiting create_machine");
        Ok(())
    }

    /// create_boot_source creates a boot source and configure it to microVM
    /// mainly used when creating root file system
    pub(super) async fn create_boot_source(
        &self,
        image_path: &PathBuf,
        initrd_path: &Option<PathBuf>,
        kernel_args: &Option<String>,
    ) -> Result<(), MachineError> {
        let bsrc = BootSource {
            kernel_image_path: image_path.to_path_buf(),
            initrd_path: initrd_path.to_owned(),
            boot_args: kernel_args.to_owned(),
        };

        self.agent.put_guest_boot_source(&bsrc).await.map_err(|e| {
            error!(target: "Machine::create_boot_source", "PutGuestBootSource: {}", e.to_string());
            MachineError::Initialize(format!("PutGuestBootSource: {}", e.to_string()))
        })?;

        debug!(target: "Machine::create_boot_source", "PutGuestBootSource successful");
        Ok(())
    }

    pub(super) async fn add_vsocks(&self) -> Result<(), MachineError> {
        if self.cfg.vsock_devices.is_none() {
            info!(target: "Machine::add_vsocks", "no virtio device socket provided, just return");
            return Ok(());
        }
        let mut err: Vec<(usize, MachineError)> = Vec::new();
        for (i, dev) in self.cfg.vsock_devices.as_ref().unwrap().iter().enumerate() {
            match self.add_vsock(dev).await {
                Ok(_) => (),
                Err(e) => err.push((i, e)),
            }
        }
        if err.is_empty() {
            return Ok(());
        }
        let mut e_string = String::new();
        for (i, e) in err {
            e_string = format!(
                "{},{{ error when putting {}-th vsock: {} }}",
                e_string,
                i,
                e.to_string()
            );
        }
        Err(MachineError::Agent(format!(
            "add_vsocks errors with: {}",
            e_string
        )))
    }

    pub(super) async fn attach_drives(&self) -> Result<(), MachineError> {
        if self.cfg.drives.is_none() {
            info!(target: "Machine::attach_drives", "no drive provided, just return");
            return Ok(());
        }
        let mut err: Vec<(usize, MachineError)> = Vec::new();
        for (i, dev) in self.cfg.drives.as_ref().unwrap().iter().enumerate() {
            match self.attach_drive(dev).await {
                Ok(_) => (),
                Err(e) => err.push((i, e)),
            }
        }
        if err.is_empty() {
            return Ok(());
        }
        let mut e_string = String::new();
        for (i, e) in err {
            e_string = format!(
                "{},{{ error when putting {}-th vsock: {} }}",
                e_string,
                i,
                e.to_string()
            );
        }
        Err(MachineError::Agent(format!(
            "add_vsocks errors with: {}",
            e_string
        )))
    }

    /// called by CreateNetworkInterfacesHandler
    pub(super) async fn create_network_interfaces(&self) -> Result<(), MachineError> {
        if self.cfg.network_interfaces.is_none() {
            info!(target: "Machine::create_network_interfaces", "no network interface provided, just return");
            return Ok(());
        }
        for (_id, iface) in self
            .cfg
            .network_interfaces
            .as_ref()
            .unwrap()
            .iter()
            .enumerate()
        {
            self.create_network_interface(iface).await?;
        }

        Ok(())
    }

    /// called by ConfigMmdsHandler
    /// set_mmds_config sets the machine's mmds system
    pub(super) async fn set_mmds_config(&self) -> Result<(), MachineError> {
        if self.cfg.mmds_address.is_none() {
            return Ok(());
        }
        let mut mmds_config = MmdsConfig::default();
        mmds_config.ipv4_address = Some(self.cfg.mmds_address.as_ref().unwrap().to_string());
        self.agent
            .put_mmds_config(&mmds_config)
            .await
            .map_err(|e| {
                error!(
                    "Setting mmds configuration failed: {}: {}",
                    self.cfg.mmds_address.as_ref().unwrap().to_string(),
                    e.to_string()
                );
                MachineError::Agent(format!(
                    "Setting mmds configuration failed: {}: {}",
                    self.cfg.mmds_address.as_ref().unwrap().to_string(),
                    e.to_string()
                ))
            })?;

        debug!("SetMmdsConfig successful");
        Ok(())
    }

    /// called by NewCreateBalloonHandler
    /// create_balloon creates a balloon device if one does not exist.
    pub(super) async fn create_balloon(&self) -> Result<(), MachineError> {
        if self.cfg.balloon.is_none() {
            return Ok(());
        }
        self.agent
            .put_balloon(self.cfg.balloon.as_ref().unwrap())
            .await
            .map_err(|e| {
                error!("Create balloon device failed: {}", e.to_string());
                MachineError::Agent(format!("Create balloon device failed: {}", e.to_string()))
            })?;

        debug!("Created balloon device successful");
        Ok(())
    }

    /// set_metadata sets the machine's metadata for MDDS
    pub(super) async fn set_metadata(&self) -> Result<(), MachineError> {
        if self.cfg.init_metadata.is_none() {
            return Ok(());
        }
        self.agent
            .put_mmds(self.cfg.init_metadata.as_ref().unwrap())
            .await
            .map_err(|e| {
                error!("Setting metadata: {}", e.to_string());
                MachineError::Agent(format!("Setting metadata: {}", e.to_string()))
            })?;

        debug!("SetMetadata successful");
        Ok(())
    }
}

/// useful methods that could be exposed to users
impl Machine {
    /// update_metadata patches the machine's metadata for MDDS
    pub async fn update_metadata(&self, metadata: &String) -> Result<(), MachineError> {
        self.agent.patch_mmds(metadata).await.map_err(|e| {
            error!("Updating metadata: {}", e.to_string());
            MachineError::Agent(format!("Updating metadata: {}", e.to_string()))
        })?;

        debug!("UpdateMetadata successful");
        Ok(())
    }

    /// get_metadata gets the machine's metadata from MDDS and unmarshals it into v
    pub async fn get_metadata(&self) -> Result<String, MachineError> {
        let res = self.agent.get_mmds().await.map_err(|e| {
            error!("Getting metadata: {}", e.to_string());
            MachineError::Agent(format!("Getting metadata: {}", e.to_string()))
        })?;

        debug!("GetMetadata successful");
        Ok(res)
    }

    /// update_guest_drive will modify the current guest drive of ID index with the new
    /// parameters of the partialDrive
    pub async fn update_guest_drive(
        &self,
        drive_id: String,
        path_on_host: PathBuf,
    ) -> Result<(), MachineError> {
        let partial_drive = PartialDrive {
            drive_id,
            path_on_host: Some(path_on_host),
            rate_limiter: None,
        };
        self.agent
            .patch_guest_drive_by_id(&partial_drive)
            .await
            .map_err(|e| {
                error!("PatchGuestDrive failed: {}", e.to_string());
                MachineError::Agent(format!("PatchGuestDrive failed: {}", e.to_string()))
            })?;

        debug!("PatchGuestDrive successful");
        Ok(())
    }

    /// describe_instance_info gets the information of the microVM.
    pub async fn describe_instance_info(&self) -> Result<InstanceInfo, MachineError> {
        let instance_info = self.agent.describe_instance().await.map_err(|e| {
            error!("Getting Instance Info: {}", e.to_string());
            MachineError::Agent(format!("Getting Instance Info: {}", e.to_string()))
        })?;

        debug!("GetInstanceInfo successful");
        Ok(instance_info)
    }

    /// create_snapshot creates a snapshot of the VM.
    pub async fn create_snapshot(
        &self,
        mem_file_path: &PathBuf,
        snapshot_path: &PathBuf,
    ) -> Result<(), MachineError> {
        let snapshot_params = SnapshotCreateParams {
            mem_file_path: mem_file_path.to_owned(),
            snapshot_path: snapshot_path.to_owned(),
            snapshot_type: None,
            version: None,
        };

        self.agent
            .create_snapshot(&snapshot_params)
            .await
            .map_err(|e| {
                error!("failed to create a snapshot of the VM: {}", e.to_string());
                MachineError::Agent(format!(
                    "failed to create a snapshot of the VM: {}",
                    e.to_string()
                ))
            })?;
        debug!("snapshot created successfully");
        Ok(())
    }

    /// get_balloon_config gets the current balloon device configuration.
    pub async fn get_balloon_config(&self) -> Result<Balloon, MachineError> {
        let balloon = self.agent.describe_balloon_config().await.map_err(|e| {
            error!("Getting balloon config: {}", e.to_string());
            MachineError::Agent(format!("Getting balloon config: {}", e.to_string()))
        })?;

        debug!("GetBalloonConfig successful");
        Ok(balloon)
    }

    /// update_balloon will update an existing balloon device, before or after machine startup.
    pub async fn update_balloon(&self, amount_mib: i64) -> Result<(), MachineError> {
        let balloon_update = BalloonUpdate { amount_mib };
        self.agent
            .patch_balloon(&balloon_update)
            .await
            .map_err(|e| {
                error!("Update balloon device failed: {}", e.to_string());
                MachineError::Agent(format!("Update balloon device failed: {}", e.to_string()))
            })?;

        debug!("Update balloon device successful");
        Ok(())
    }

    /// get_balloon_stats gets the latest balloon device statistics, only if enabled pre-boot.
    pub async fn get_balloon_stats(&self) -> Result<BalloonStatistics, MachineError> {
        let balloon_stats = self.agent.describe_balloon_stats().await.map_err(|e| {
            error!("Getting balloonStats: {}", e.to_string());
            MachineError::Agent(format!("Getting balloonStats: {}", e.to_string()))
        })?;

        debug!("GetBalloonStats successful");
        Ok(balloon_stats)
    }

    /// update_balloon_stats will update a balloon device statistics polling interval.
    pub async fn update_balloon_stats(
        &self,
        stats_polling_interval_s: i64,
    ) -> Result<(), MachineError> {
        let balloon_stats_update = BalloonStatsUpdate {
            stats_polling_interval_s,
        };
        self.agent
            .patch_balloon_stats_interval(&balloon_stats_update)
            .await
            .map_err(|e| {
                error!("UpdateBalloonStats failed: {}", e.to_string());
                MachineError::Agent(format!("UpdateBalloonStats failed: {}", e.to_string()))
            })?;

        debug!("UpdateBalloonStats successful");
        Ok(())
    }

    /// update_guest_network_interface_rate_limit modifies the specified network interface's rate limits
    pub async fn update_guest_network_interface_rate_limit(
        &self,
        iface_id: String,
        rate_limiters: RateLimiterSet,
    ) -> Result<(), MachineError> {
        let iface = PartialNetworkInterface {
            iface_id: iface_id.to_owned(),
            rx_rate_limiter: rate_limiters.in_rate_limiter,
            tx_rate_limiter: rate_limiters.out_rate_limiter,
        };

        self.agent
            .patch_guest_network_interface_by_id(&iface)
            .await
            .map_err(|e| {
                error!(
                    "Update network interface failed: {}: {}",
                    iface_id,
                    e.to_string()
                );
                MachineError::Agent(format!(
                    "Update network interface failed: {}: {}",
                    iface_id,
                    e.to_string()
                ))
            })?;

        info!("Updated network interface");
        Ok(())
    }

    /// refresh_machine_configuration synchronizes our cached representation of the machine configuration
    /// with that reported by the Firecracker API
    pub async fn refresh_machine_configuration(&mut self) -> Result<(), MachineError> {
        debug!(target: "Machine::refresh_machine_configuration", "called Machine::refresh_machine_configuration");
        let machine_config = self.agent.get_machine_configuration().await.map_err(|e| {
            error!(target: "Machine::refresh_machine_configuration", "unable to inspect firecracker MachineConfiguration: {}", e);
            MachineError::Agent(format!(
                "unable to inspect firecracker MachineConfiguration: {}",
                e.to_string()
            ))
        })?;

        debug!(target: "Machine::refresh_machine_configuration", "got: {:#?}", machine_config);
        self.machine_config = machine_config;
        Ok(())
    }

    pub async fn get_export_vm_config(&self) -> Result<FullVmConfiguration, MachineError> {
        debug!(target: "Machine::get_export_vm_config", "called Machine::get_export_vm_config");
        let config: FullVmConfiguration = self.agent.get_export_vm_config().await.map_err(|e| {
            error!(target: "Machine::get_export_vm_config", "unable to inspect vm config: {}", e);
            MachineError::Agent(format!("unable to inspect vm config: {}", e.to_string()))
        })?;

        Ok(config)
    }

    pub async fn get_firecracker_version(&mut self) -> Result<FirecrackerVersion, MachineError> {
        debug!(target: "Machine::get_firecracker_version", "called Machine::get_firecracker_version");
        let ver = self.agent.get_firecracker_version().await.map_err(|e| {
            error!(target: "Machine::get_firecracker_version", "unable to inspect firecracker version: {}", e);
            MachineError::Agent(format!(
                "unable to inspect firecracker version: {}",
                e.to_string()
            ))
        })?;

        Ok(ver)
    }

    pub async fn update_machine_configuration(
        &mut self,
        machine_config: &MachineConfiguration,
    ) -> Result<(), MachineError> {
        debug!(target: "Machine::update_machine_configuration", "called Machine::update_machine_configuration");
        self.agent.patch_machine_configuration(machine_config).await.map_err(|e| {
            error!(target: "Machine::update_machine_configuration", "unable to update machine configuration: {}", e);
            MachineError::Agent(format!(
                "unable to update machine configuration: {}",
                e.to_string()
            ))
        })
    }

    pub async fn load_from_snapshot(
        &mut self,
        snapshot_load_params: &SnapshotLoadParams,
    ) -> Result<(), MachineError> {
        debug!(target: "Machine::load_from_snapshot", "called Machine::load_from_snapshot");
        self.agent
            .load_snapshot(snapshot_load_params)
            .await
            .map_err(|e| {
                error!(target: "Machine::load_from_snapshot", "unable to load snapshot: {}", e);
                MachineError::Agent(format!("unable to load snapshot: {}", e.to_string()))
            })
    }
}


pub mod test_utils {
    use std::{collections::HashMap, path::PathBuf};

    use log::info;

    use crate::{
        model::{drive::Drive, vsock::Vsock},
        utils::{make_socket_path, TestArgs},
    };

    use super::{Config, Machine, MachineError};

    // expose start_vmm api to test modules
    impl Machine {
        pub async fn start_vmm_test(&mut self) -> Result<(), MachineError> {
            self.start_vmm().await
        }
    }

    pub async fn test_create_machine(m: &mut Machine) -> Result<(), MachineError> {
        m.create_machine().await?;
        Ok(())
    }

    pub fn test_machine_config_application(
        m: &mut Machine,
        expected_values: &Config,
    ) -> Result<(), MachineError> {
        assert_eq!(
            expected_values.machine_cfg.as_ref().unwrap().vcpu_count,
            m.machine_config.vcpu_count
        );
        assert_eq!(
            expected_values.machine_cfg.as_ref().unwrap().mem_size_mib,
            m.machine_config.mem_size_mib
        );
        Ok(())
    }

    pub async fn test_create_boot_source(
        m: &mut Machine,
        vmlinux_path: &PathBuf,
    ) -> Result<(), MachineError> {
        // panic=0: This option disables reboot-on-panic behavior for the kernel. We
        //          use this option as we might run the tests without a real root
        //          filesystem available to the guest.
        // Kernel command-line options can be found in the kernel source tree at
        // Documentation/admin-guide/kernel-parameters.txt.
        let kernel_args = "ro console=ttyS0 noapic reboot=k panic=0 pci=off nomodules".to_string();
        m.create_boot_source(vmlinux_path, &Some("".into()), &Some(kernel_args))
            .await?;

        Ok(())
    }

    pub async fn test_update_guest_drive(m: &mut Machine) -> Result<(), MachineError> {
        let path = TestArgs::test_data_path().join("drive-3.img");
        m.update_guest_drive("2".to_string(), path).await?;
        Ok(())
    }

    pub async fn test_attach_root_drive(m: &mut Machine) -> Result<(), MachineError> {
        let drive = Drive {
            drive_id: "0".to_string(),
            is_root_device: true,
            is_read_only: true,
            path_on_host: TestArgs::test_root_fs(),
            partuuid: None,
            cache_type: None,
            rate_limiter: None,
            io_engine: None,
            socket: None,
        };
        m.attach_drive(&drive).await?;

        Ok(())
    }

    pub async fn test_attch_secondary_drive(m: &mut Machine) -> Result<(), MachineError> {
        let drive = Drive {
            drive_id: "0".to_string(),
            is_root_device: true,
            is_read_only: true,
            path_on_host: TestArgs::test_root_fs(),
            partuuid: None,
            cache_type: None,
            rate_limiter: None,
            io_engine: None,
            socket: None,
        };
        m.attach_drive(&drive).await?;
        Ok(())
    }

    pub async fn test_attach_vsock(m: &mut Machine) -> Result<(), MachineError> {
        let time_stamp = std::time::SystemTime::now().elapsed().unwrap().as_nanos();
        let dev = Vsock {
            vsock_id: Some("1".to_string()),
            guest_cid: 3,
            uds_path: [time_stamp.to_string(), ".vsock".to_string()]
                .iter()
                .collect(),
        };
        m.add_vsock(&dev).await?;

        Ok(())
    }

    pub async fn test_start_instance(m: &mut Machine) -> Result<(), MachineError> {
        m.start_instance().await?;
        Ok(())
    }

    pub async fn test_stop_vmm(m: &mut Machine) -> Result<(), MachineError> {
        m.stop_vmm().await?;
        Ok(())
    }

    pub async fn test_shutdown(m: &mut Machine) -> Result<(), MachineError> {
        m.shutdown().await?;
        Ok(())
    }

    pub async fn test_wait_for_socket() -> Result<(), MachineError> {
        let socket_path = make_socket_path("test_wait_for_socket");
        // let (_sig_send, sig_recv) = async_channel::bounded(64);
        let cfg = Config {
            socket_path: Some(socket_path),
            ..Default::default()
        };
        let m = Machine::new(cfg)?;
        m.wait_for_socket(10.0).await?;
        Ok(())
    }

    pub async fn test_set_metadata(m: &mut Machine) -> Result<(), MachineError> {
        let mut metadata = HashMap::new();
        metadata.insert("key", "value");

        let s = serde_json::to_string(&metadata).map_err(|e| {
            MachineError::Execute(format!("fail to serialize HashMap: {}", e.to_string()))
        })?;
        m.cfg.init_metadata = Some(s);
        m.set_metadata().await?;
        Ok(())
    }

    pub async fn test_update_metadata(m: &mut Machine) -> Result<(), MachineError> {
        let mut metadata = HashMap::new();
        metadata.insert("patch_key", "patch_value");

        let s = serde_json::to_string(&metadata).map_err(|e| {
            MachineError::Execute(format!("fail to serialize HashMap: {}", e.to_string()))
        })?;
        m.update_metadata(&s).await?;
        Ok(())
    }

    pub async fn test_get_metadata(m: &mut Machine) -> Result<(), MachineError> {
        let s: String = m.get_metadata().await?;
        info!("get_metadata: {}", s);
        Ok(())
    }

    pub async fn test_get_instance_info(m: &mut Machine) -> Result<(), MachineError> {
        let instance = m.describe_instance_info().await?;
        if instance.app_name == "".to_string() {
            Err(MachineError::Execute(
                "invalid instance app name".to_string(),
            ))
        } else if instance.id == "".to_string() {
            Err(MachineError::Execute("invalid instance id".to_string()))
        } else if instance.vmm_version == "".to_string() {
            Err(MachineError::Execute("invalid vmm version".to_string()))
        } else {
            Ok(())
        }
    }

    // pub async fn test_log_files()

    pub async fn test_socket_path_set() -> Result<(), MachineError> {
        let socket_path: PathBuf = "foo/bar".into();
        // let (_sig_send, sig_recv) = async_channel::bounded(64);
        let cfg = Config {
            socket_path: Some(socket_path.to_owned()),
            ..Default::default()
        };
        let m = Machine::new(cfg)?;
        let mut found = false;
        let mut iter = m.cmd.as_ref().unwrap().as_std().get_args();
        loop {
            let s = iter.next();
            if s.is_none() {
                break;
            } else if s.unwrap() != "--api-sock" {
                continue;
            } else {
                found = true;
                let arg = iter.next();
                if arg.is_none() {
                    return Err(MachineError::Initialize(format!(
                        "no socket path provided after `--api-sock`"
                    )));
                } else if arg.unwrap() != socket_path {
                    return Err(MachineError::Initialize(format!(
                        "Incorrect socket path: {:#?}",
                        arg.unwrap()
                    )));
                }
                break;
            }
        }
        if !found {
            return Err(MachineError::Initialize(
                "fail to find socket path".to_string(),
            ));
        }
        Ok(())
    }

    pub async fn test_pid() -> Result<(), MachineError> {
        let cfg = Config::default().set_disable_validation(true);
        let mut m = Machine::new(cfg)?;
        m.start().await?;
        println!("{}", m.pid()?);
        m.stop_vmm().await?;
        Ok(())
    }
}
