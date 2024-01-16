use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{model, };

use super::{handlers::Handlers, agent::Agent, jailer::{JailerConfig, StdioTypes}, connection_pool::SocketConnectionPool};


type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

const USER_AGENT: &'static str = "rustfire";

// as specified in http://man7.org/linux/man-pages/man8/ip-netns.8.html
const DEFAULT_NETNS_DIR: &'static str = "/var/run/netns";

// env name to make firecracker init timeout configurable
const FIRECRACKER_INIT_TIMEOUT_ENV: &'static str = "RUSTFIRE_INIT_TIMEOUT_SECONDS";

const DEFAULT_FIRECRACKER_INIT_TIMEOUT_SECONDS: usize = 3;

type SeccompLevelValue = usize;

// SeccompLevelDisable is the default value.
const SECCOMP_LEVEL_DISABLE: SeccompLevelValue = 0;

// SeccompLevelBasic prohibits syscalls not whitelisted by Firecracker.
const SECCOMP_LEVEL_BASIC: SeccompLevelValue = 1;

// SeccompLevelAdvanced adds further checks on some of the parameters of the
// allowed syscalls.
const SECCOMP_LEVEL_ADVANCED: SeccompLevelValue = 2;

// Config is a collection of user-configurable VMM settings
// #[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    // SocketPath defines the file path where the Firecracker control socket
	// should be created.
    pub(crate) socket_path: Option<PathBuf>,

    // LogPath defines the file path where the Firecracker log is located.
    pub(crate) log_path: Option<PathBuf>,

    // LogFifo defines the file path where the Firecracker log named-pipe should
	// be located.
    pub(crate) log_fifo: Option<PathBuf>,

    // LogLevel defines the verbosity of Firecracker logging.  Valid values are
	// "Error", "Warning", "Info", and "Debug", and are case-sensitive.
    log_level: model::logger::LogLevel,

    // MetricsPath defines the file path where the Firecracker metrics
	// is located.
    pub(crate) metrics_path: Option<PathBuf>,

    // MetricsFifo defines the file path where the Firecracker metrics
	// named-pipe should be located.
    pub(crate) metrics_fifo: Option<PathBuf>,

    // KernelImagePath defines the file path where the kernel image is located.
	// The kernel image must be an uncompressed ELF image.
    pub(crate) kernel_image_path: PathBuf,

    // InitrdPath defines the file path where initrd image is located.
	//
	// This parameter is optional.
    pub(crate) initrd_path: Option<PathBuf>,

    // KernelArgs defines the command-line arguments that should be passed to
	// the kernel.
    kernel_args: String,

    // Drives specifies BlockDevices that should be made available to the
	// microVM.
    pub(crate) drives: Vec<model::drive::Drive>,

    // NetworkInterfaces specifies the tap devices that should be made available
	// to the microVM.
    network_interfaces: model::network_interface::NetworkInterface,

    // FifoLogWriter is an io.Writer(Stdio) that is used to redirect the contents of the
	// fifo log to the writer.
    // pub(crate) fifo_log_writer: Option<std::process::Stdio>,
    pub(crate) fifo_log_writer: Option<StdioTypes>,

    // VsockDevices specifies the vsock devices that should be made available to
	// the microVM.
    vsock_devices: Vec<model::vsock::Vsock>,

    // MachineCfg represents the firecracker microVM process configuration
    machine_cfg: model::machine_configuration::MachineConfiguration,
    
    // DisableValidation allows for easier mock testing by disabling the
	// validation of configuration performed by the SDK(crate).
    disable_validation: bool,

    // JailerCfg is configuration specific for the jailer process.
    pub(crate) jailer_cfg: Option<JailerConfig>,

    // (Optional) VMID is a unique identifier for this VM. It's set to a
	// random uuid if not provided by the user. It's used to set Firecracker's instance ID.
	// If CNI configuration is provided as part of NetworkInterfaces,
	// the VMID is used to set CNI ContainerID and create a network namespace path.
    vmid: Option<String>,

    // NetNS represents the path to a network namespace handle. If present, the
	// application will use this to join the associated network namespace
    pub(crate) net_ns: Option<String>,

    // ForwardSignals is an optional list of signals to catch and forward to
	// firecracker. If not provided, the default signals will be used.
    // forward_signals: Vec<>,

    // SeccompLevel specifies whether seccomp filters should be installed and how
	// restrictive they should be. Possible values are:
	//
	//	0 : (default): disabled.
	//	1 : basic filtering. This prohibits syscalls not whitelisted by Firecracker.
	//	2 : advanced filtering. This adds further checks on some of the
	//			parameters of the allowed syscalls.
    pub(crate) seccomp_level: SeccompLevelValue,

    // MmdsAddress is IPv4 address used by guest applications when issuing requests to MMDS.
	// It is possible to use a valid IPv4 link-local address (169.254.0.0/16).
	// If not provided, the default address (169.254.169.254) will be used.
    mmds_address: std::net::Ipv4Addr,
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        todo!()
    }

    pub fn validate_network(&self) -> Result<()> {
        if self.disable_validation {
            return Ok(())
        } else {
            todo!()
            //return cfg.NetworkInterfaces.validate(parseKernelArgs(cfg.KernelArgs))
        }
    }
}

pub struct Machine {
    pub(crate) handlers: Handlers,

    pub(crate) cfg: Config,
    client: SocketConnectionPool,
    pub(crate) cmd: std::process::Command,
    logger: crate::model::logger::Logger,
    
    // The actual machine config as reported by Firecracker
    // id est, not the config set by user, which should be a field of `cfg`
    machine_config: crate::model::machine_configuration::MachineConfiguration,
    
    // startOnce ensures that the machine can only be started once
    start_once: std::sync::Once,

    // exitCh is a channel which gets closed when the VMM exits
    exit_ch: (),
    
    // fatalErr records an error that either stops or prevent starting the VMM
    fatalerr: Option<GenericError>,

    // callbacks that should be run when the machine is being torn down
    cleanup_once: std::sync::Once,

    cleanup_funcs: Vec<Box<dyn FnOnce() -> Result<()>>>,
}

#[async_trait]
pub trait MachineInterface {
    async fn start() -> Result<()>;
    async fn stop_vmm() -> Result<()>;
    async fn shutdown() -> Result<()>;
    async fn wait() -> Result<()>;
    async fn set_metadata(s: String) -> Result<()>;
    async fn update_guest_drive(s1: String, s2: String) -> Result<()>;
    async fn update_guest_network_interface_rate_limit(s: String) -> Result<()>;
}

// RateLimiterSet represents a pair of RateLimiters (inbound and outbound)
pub struct RateLimiterSet {
    // InRateLimiter limits the incoming bytes.
    in_rate_limiter: model::rate_limiter::RateLimiter,

    // OutRateLimiter limits the outgoing bytes.
    out_rate_limiter: model::rate_limiter::RateLimiter,
}


impl Machine {
    // Logger returns a logrus logger appropriate for logging hypervisor messages
    // pub(crate) fn logger(&self, );

    // PID returns the machine's running process PID or an error if not running
    

    // NewMachine initializes a new Machine instance and performs validation of the
    // provided Config.
    pub fn new_machine(mut cfg: Config) -> Result<Machine> {
        // 创建一个与机器交互的channel
        
        // 为机器设置vmid参数
        if cfg.vmid.is_none() {
            let random_id = uuid::Uuid::new_v4().to_string();
            cfg.vmid = Some(random_id);
        }

        // let mut m_handlers = DEFAULT_HANDLERS;

        // if cfg.jailer_cfg.is_some() {
        //     // m_handlers.validation.push(JailerConfigValidationHandler);

        // }
        todo!()
    }
}