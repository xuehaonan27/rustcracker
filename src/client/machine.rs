use std::{net::Ipv4Addr, path::PathBuf, sync::Once};

use async_trait::async_trait;

use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot::Receiver;

use crate::{
    client::{
        command_builder::VMMCommandBuilder,
        handler::{
            CleaningUpSocketHandlerName, Handler, ValidateCfgHandlerName,
            ValidateJailerCfgHandlerName,
        },
        jailer::jail,
    },
    model::{
        self,
        balloon::Balloon,
        balloon_stats::BalloonStatistics,
        balloon_stats_update::BalloonStatsUpdate,
        balloon_update::BalloonUpdate,
        boot_source::BootSource,
        drive::Drive,
        instance_action_info::InstanceActionInfo,
        instance_info::InstanceInfo,
        kernel_args::KernelArgs,
        logger::{LogLevel, Logger},
        machine_configuration::MachineConfiguration,
        metrics::Metrics,
        mmds_config::MmdsConfig,
        network_interface::NetworkInterface,
        partial_drive::PartialDrive,
        partial_network_interface::PartialNetworkInterface,
        rate_limiter::RateLimiterSet,
        snapshot_create_params::SnapshotCreateParams,
        vm::{VM_STATE_PAUSED, VM_STATE_RESUMED},
        vsock::Vsock,
    },
    utils::Metadata,
};

use super::{
    agent::Agent,
    handler::{HandlerList, Handlers},
    jailer::{JailerConfig, StdioTypes},
    signals::Signal,
};

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

/// Config is a collection of user-configurable VMM settings
/// describe the microVM
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    // SocketPath defines the file path where the Firecracker control socket
    // should be created.
    pub socket_path: Option<PathBuf>,

    // LogPath defines the file path where the Firecracker log is located.
    pub log_path: Option<PathBuf>,

    // LogFifo defines the file path where the Firecracker log named-pipe should
    // be located.
    pub log_fifo: Option<PathBuf>,

    // LogLevel defines the verbosity of Firecracker logging.  Valid values are
    // "Error", "Warning", "Info", and "Debug", and are case-sensitive.
    pub log_level: Option<LogLevel>,

    // MetricsPath defines the file path where the Firecracker metrics
    // is located.
    pub metrics_path: Option<PathBuf>,

    // MetricsFifo defines the file path where the Firecracker metrics
    // named-pipe should be located.
    pub metrics_fifo: Option<PathBuf>,

    // KernelImagePath defines the file path where the kernel image is located.
    // The kernel image must be an uncompressed ELF image.
    pub kernel_image_path: Option<PathBuf>,

    // InitrdPath defines the file path where initrd image is located.
    //
    // This parameter is optional.
    pub initrd_path: Option<PathBuf>,

    // KernelArgs defines the command-line arguments that should be passed to
    // the kernel.
    pub kernel_args: Option<String>,

    // Drives specifies BlockDevices that should be made available to the
    // microVM.
    pub drives: Option<Vec<Drive>>,

    // NetworkInterfaces specifies the tap devices that should be made available
    // to the microVM.
    pub network_interfaces: Option<Vec<NetworkInterface>>,

    // FifoLogWriter is an io.Writer(Stdio) that is used to redirect the contents of the
    // fifo log to the writer.
    // pub(crate) fifo_log_writer: Option<std::process::Stdio>,
    pub fifo_log_writer: Option<StdioTypes>,

    // VsockDevices specifies the vsock devices that should be made available to
    // the microVM.
    pub vsock_devices: Option<Vec<Vsock>>,

    // MachineCfg represents the firecracker microVM process configuration
    pub machine_cfg: Option<MachineConfiguration>,

    // DisableValidation allows for easier mock testing by disabling the
    // validation of configuration performed by the SDK(crate).
    pub disable_validation: bool,

    // JailerCfg is configuration specific for the jailer process.
    pub jailer_cfg: Option<JailerConfig>,

    // (Optional) VMID is a unique identifier for this VM. It's set to a
    // random uuid if not provided by the user. It's used to set Firecracker's instance ID.
    // If CNI configuration is provided as part of NetworkInterfaces,
    // the VMID is used to set CNI ContainerID and create a network namespace path.
    pub vmid: Option<String>,

    // NetNS represents the path to a network namespace handle. If present, the
    // application will use this to join the associated network namespace
    pub net_ns: Option<String>,

    // ForwardSignals is an optional list of signals to catch and forward to
    // firecracker. If not provided, the default signals will be used.
    pub forward_signals: Option<Vec<Signal>>,

    // SeccompLevel specifies whether seccomp filters should be installed and how
    // restrictive they should be. Possible values are:
    //
    //	0 : (default): disabled.
    //	1 : basic filtering. This prohibits syscalls not whitelisted by Firecracker.
    //	2 : advanced filtering. This adds further checks on some of the
    //			parameters of the allowed syscalls.
    pub seccomp_level: Option<SeccompLevelValue>,

    // MmdsAddress is IPv4 address used by guest applications when issuing requests to MMDS.
    // It is possible to use a valid IPv4 link-local address (169.254.0.0/16).
    // If not provided, the default address (169.254.169.254) will be used.
    pub mmds_address: Option<std::net::Ipv4Addr>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            socket_path: None,
            log_path: None,
            log_fifo: None,
            log_level: None,
            metrics_path: None,
            metrics_fifo: None,
            kernel_image_path: None,
            initrd_path: None,
            kernel_args: None,
            drives: None,
            network_interfaces: None,
            fifo_log_writer: None,
            vsock_devices: None,
            machine_cfg: None,
            disable_validation: false,
            jailer_cfg: None,
            vmid: None,
            net_ns: None,
            seccomp_level: None,
            mmds_address: None,
            forward_signals: None,
        }
    }
}

impl Config {
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
        }

        if self.machine_cfg.is_none() {
            return Err(MachineError::Validation(
                "no machine configuration provided".to_string(),
            ));
        } else {
            if self.machine_cfg.as_ref().unwrap().get_vcpu_count() < 1 {
                return Err(MachineError::Validation(
                    "machine needs a non-zero vcpu count".to_string(),
                ));
            }
            if self.machine_cfg.as_ref().unwrap().get_mem_size_in_mib() < 1 {
                return Err(MachineError::Validation(
                    "machine needs a non-zero amount of memory".to_string(),
                ));
            }
        }

        Ok(())
    }

    pub fn validate_network(&self) -> Result<(), MachineError> {
        if self.disable_validation {
            return Ok(());
        } else {
            todo!()
            //return cfg.NetworkInterfaces.validate(parseKernelArgs(cfg.KernelArgs))
        }
    }

    pub fn with_machine_config(mut self, machine_config: MachineConfiguration) -> Self {
        self.machine_cfg = Some(machine_config);
        self
    }

    pub fn set_disable_validation(mut self, b: bool) -> Self {
        self.disable_validation = b;
        self
    }
}

/// Machine is process handler of rust side
pub struct Machine {
    pub(crate) handlers: Handlers,

    pub(crate) cfg: Config,

    agent: Agent,

    pub(crate) cmd: Option<tokio::process::Command>,

    child_process: Option<tokio::process::Child>,

    pid: Option<u32>,

    logger: Option<env_logger::Builder>,

    // The actual machine config as reported by Firecracker
    // id est, not the config set by user, which should be a field of `cfg`
    machine_config: MachineConfiguration,

    // startOnce ensures that the machine can only be started once
    start_once: std::sync::Once,

    // exitCh is a channel which gets closed when the VMM exits
    // implemented with sync::mpsc, which will receive the instruction
    // sent by outside
    exit_ch: tokio::sync::oneshot::Receiver<MachineMessage>,

    // fatalErr records an error that either stops or prevent starting the VMM
    fatalerr: Option<MachineError>,

    // callbacks that should be run when the machine is being torn down
    cleanup_once: std::sync::Once,

    cleanup_funcs: HandlerList,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MachineMessage {
    StopVMM,
}

#[derive(thiserror::Error, Debug)]
pub enum MachineError {
    /// Mostly problems related to directories error or unavailable files
    #[error("Could not set up environment(e.g. file, linking) the machine, reason: {0}")]
    FileError(String),
    /// Failure when validating the configuration before starting the microVM
    #[error("Invalid configuration for the machine, reason: {0}")]
    Validation(String),
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

#[async_trait]
pub trait MachineTrait {
    async fn start() -> Result<(), MachineError>;
    async fn stop_vmm() -> Result<(), MachineError>;
    async fn shutdown() -> Result<(), MachineError>;
    async fn wait() -> Result<(), MachineError>;
    async fn set_metadata(s: String) -> Result<(), MachineError>;
    async fn update_guest_drive(s1: String, s2: String) -> Result<(), MachineError>;
    async fn update_guest_network_interface_rate_limit(s: String) -> Result<(), MachineError>;
}

/// start, start_instance和start_vmm的区别
/// start是外部应该调用的方法, 它会调用start_instance并且消耗Machine的Once, 保证每一个Machine实例只被启动一次.
/// start_instance仅仅发送InstanceActionInfo::instance_start()给microVM, 不应该被外部直接调用, 仅仅由start调用
/// 在此之前需要start_vmm为其配置好环境然后启动firecracker进程
///
/// 总结: 先调用start_vmm, 然后调用start
///
/// shutdown, stop_vmm的区别
/// shutdown仅仅发送InstanceActionInfo::send_ctrl_alt_del()给microVM.
/// stop_vmm停止firecracker并做好收尾工作
///
/// 总结: 先调用shutdown, 然后调用firecracker.
/// 和go实现不同的是, stop_vmm没有选择SIGTERM而是SIGKILL.
/// 保证绝对终止firecracker进程.
///
/// 可以使用tokio::select!和channel完成exit_ch的编写
///
/// 将所有put操作全部pub(super)防止user直接调用
impl Machine {
    /// default returns a blanck machine which should be configured
    /// and one should never call this function so set as private.
    /// The reason why I do not want to impl Default for Machine
    /// is the same. Just keep it private.
    fn default(recv: Receiver<MachineMessage>) -> Self {
        Machine {
            handlers: Handlers::default(),
            cfg: Config::default(),
            agent: Agent::blank(),
            cmd: None,
            child_process: None,
            pid: None,
            logger: None,
            machine_config: MachineConfiguration::default(),
            start_once: Once::new(),
            exit_ch: recv,
            fatalerr: None,
            cleanup_once: Once::new(),
            cleanup_funcs: HandlerList::blank(),
        }
    }

    /// new initializes a new Machine instance and performs validation of the
    /// provided Config.
    pub fn new(
        mut cfg: Config,
        recv: Receiver<MachineMessage>,
        agent_request_timeout: u64,
        agent_init_timeout: u64,
    ) -> Result<Machine, MachineError> {
        // create a channel for communicating with microVM (stopping microVM)
        let mut machine = Self::default(recv);

        // set vmid for microVM
        if cfg.vmid.is_none() {
            let random_id = uuid::Uuid::new_v4().to_string();
            cfg.vmid = Some(random_id);
        }

        // set default handlers for microVM
        // let mut machine_handlers = Handlers::default();

        if cfg.jailer_cfg.is_some() {
            // jailing the microVM if jailer config provided
            // validate jailer config
            machine
                .handlers
                .validation
                .append(vec![Handler::JailerConfigValidationHandler {
                    name: ValidateJailerCfgHandlerName,
                }]);
            // jail the machine
            jail(&mut machine, &mut cfg)?;
        } else {
            // microVM without jailer
            machine
                .handlers
                .validation
                .append(vec![Handler::ConfigValidationHandler {
                    name: ValidateCfgHandlerName,
                }]);

            // TODO: another command building process
            // machine.cmd
            let c = VMMCommandBuilder::default()
                .with_socket_path(cfg.socket_path.as_ref().unwrap())
                .with_args(vec![
                    "--seccomp-level".to_string(),
                    cfg.seccomp_level
                        .unwrap_or(SECCOMP_LEVEL_DISABLE)
                        .to_string(),
                    "--id".to_string(),
                    cfg.vmid.as_ref().unwrap().to_string(),
                ])
                .build();
            machine.cmd = Some(c.into());
        }

        if machine.logger.is_none() {
            let logger = env_logger::builder();
            machine.logger = Some(logger);
        }

        machine.agent = Agent::new(
            cfg.socket_path.as_ref().ok_or(MachineError::Initialize(
                "no socket_path provided in the config".to_string(),
            ))?,
            agent_request_timeout,
            agent_init_timeout,
        );

        // TODO: forward_signals
        if cfg.forward_signals.is_none() {
            cfg.forward_signals = Some(vec![
                Signal::SIGINT,
                Signal::SIGQUIT,
                Signal::SIGTERM,
                Signal::SIGHUP,
                Signal::SIGABRT,
            ]);
        }

        machine.machine_config = cfg
            .machine_cfg
            .as_ref()
            .ok_or(MachineError::Initialize(
                "no machine_config provided in the config".to_string(),
            ))?
            .to_owned();

        machine.cfg = cfg;

        // TODO: netns setting
        /*
        if cfg.NetNS == "" && cfg.NetworkInterfaces.cniInterface() != nil {
            m.Cfg.NetNS = m.defaultNetNSPath()
        }
        */
        todo!();

        Ok(machine)
    }

    /// logger set a appropriate logger for logging hypervisor message
    pub fn logger(&mut self) {
        let logger = env_logger::Builder::new();
        self.logger = Some(logger);
    }

    /// PID returns the machine's running process PID or an error if not running
    #[allow(non_snake_case)]
    pub fn PID(&self) -> Result<u32, MachineError> {
        if self.cmd.is_none() || self.child_process.is_none() {
            return Err(MachineError::Execute("machine is not running".to_string()));
        }

        // 如果exit_ch有消息, 说明要求停止了
        // todo!(); // "machine process has exited"

        self.child_process
            .as_ref()
            .unwrap()
            .id()
            .ok_or(MachineError::Execute(
                "machine may by not running or already stopped".to_string(),
            ))
    }

    /// Start actually start a Firecracker microVM.
    /// The context must not be cancelled while the microVM is running.
    ///
    /// It will iterate through the handler list and call each handler. If an
    /// error occurred during handler execution, that error will be returned. If the
    /// handlers succeed, then this will start the VMM instance.
    /// Start may only be called once per Machine.  Subsequent calls will return
    /// ErrAlreadyStarted.
    pub async fn start(&self) -> Result<(), MachineError> {
        debug!("Called Machine::start");
        let mut already_started = true;
        self.start_once.call_once(|| {
            debug!("marking Machine as Started");
            already_started = false;
        });
        if already_started {
            return Err(MachineError::Execute("machine already started".to_string()));
        }
        // do clean up
        todo!();

        // run functions according to handlers
        todo!();

        self.start_instance().await?;
        Ok(())
    }

    /// wait will wait until the firecracker process has finished.
    /// it will deliver the same error to all subscribers.
    async fn wait(&self) -> Result<(), MachineError> {
        debug!("called Machine::wait");
        if self.cmd.is_none() || self.child_process.is_none() {
            return Err(MachineError::Execute(
                "cannot wait before machine starts".to_string(),
            ));
        }
        // self.child_process.as_ref().unwrap().wait_with_output();
        todo!()
    }

    async fn start_instance(&self) -> Result<(), MachineError> {
        debug!("called Machine::start_instance");
        self.agent
            .create_sync_action(&InstanceActionInfo::instance_start())
            .await
            .map_err(|e| MachineError::Execute(e.to_string()))?;
        Ok(())
    }

    /// shutdown requests a clean shutdown of the VM by sending CtrlAltDelete on the virtual keyboard
    pub async fn shutdown(&self) -> Result<(), MachineError> {
        debug!("called Machine::shutdown");
        self.send_ctrl_alt_del().await
    }

    pub async fn send_ctrl_alt_del(&self) -> Result<(), MachineError> {
        debug!("called Machine::send_ctrl_alt_del");
        self.agent
            .create_sync_action(&InstanceActionInfo::send_ctrl_alt_del())
            .await
            .map_err(|e| MachineError::Execute(e.to_string()))?;
        Ok(())
    }

    pub async fn pause(&self) -> Result<(), MachineError> {
        debug!("called Machine::pause");
        self.agent
            .patch_vm(&VM_STATE_PAUSED)
            .await
            .map_err(|e| MachineError::Execute(e.to_string()))?;
        Ok(())
    }

    pub async fn resume(&self) -> Result<(), MachineError> {
        debug!("called Machine::resume");
        self.agent
            .patch_vm(&VM_STATE_RESUMED)
            .await
            .map_err(|e| MachineError::Execute(e.to_string()))?;
        Ok(())
    }

    /// start_vmm starts the firecracker vmm process and configures logging.
    pub(super) async fn start_vmm(&mut self) -> Result<(), MachineError> {
        if self.cfg.socket_path.is_none() {
            error!("start_vmm: no socket path provided");
            return Err(MachineError::FileError(
                "start_vmm: no socket path provided".to_string(),
            ));
        }
        info!(
            "Called start_vmm, setting up a VMM on {}",
            self.cfg.socket_path.as_ref().unwrap().display()
        );

        if self.cmd.is_none() {
            error!("start_vmm: no command provided");
            return Err(MachineError::Execute(
                "start_vmm: no command provided".to_string(),
            ));
        }
        debug!("start_vmm: starting {:#?}", self.cmd.as_ref().unwrap());

        let start_result;

        if self.cfg.net_ns.is_some() && self.cfg.jailer_cfg.is_none() {
            // If the VM needs to be started in a netns but no jailer netns was configured,
            // start the vmm child process in the netns directly here.
            todo!();
            // 这里有对于netns的设置, 然后启动进程
            start_result = self.cmd.as_mut().unwrap().spawn();
        } else {
            // Else, just start the process normally as it's either not in a netns or will
            // be placed in one by the jailer process instead.
            start_result = self.cmd.as_mut().unwrap().spawn();
            // 并且在Machine里面存储pid
        }

        if let Err(e) = start_result {
            error!("start_vmm: Failed to start vmm: {}", e.to_string());
            self.fatalerr = Some(MachineError::Execute(format!(
                "start_vmm: Failed to start vmm: {}",
                e.to_string()
            )));
            self.exit_ch.close();

            return Err(MachineError::Execute(format!(
                "start_vmm: Failed to start vmm: {}",
                e.to_string()
            )));
        } else {
            self.child_process = Some(start_result.unwrap());
        }
        debug!(
            "start_vmm: VMM started socket path is: {}",
            self.cfg.socket_path.as_ref().unwrap().display()
        );

        // add a handler that could clean up the socket file
        self.cleanup_funcs
            .append(vec![Handler::CleaningUpSocketHandler {
                name: CleaningUpSocketHandlerName,
                socket_path: self.cfg.socket_path.as_ref().unwrap().to_path_buf(),
            }]);

        // 一个显式地单独的协程等待进程结束. 可以用std::process下面的wait等等.
        // 查看退出码
        // 逐个call保存的clean_up_funcs

        // 本协程继续
        self.setup_signals().await?;
        debug!("start_vmm: signals set successful");
        self.wait_for_socket(self.agent.firecracker_init_timeout)
            .await?;
        todo!(); // setup_signals
                 // Wait for firecracker to initialize:

        // 再生成一个协程, 如果有通道发送了一个结束命令消息, 就直接stop_vmm

        // 再生成一个协程, 向订阅者发送结束的消息

        // 所以其实Machine里面需要两个通道

        debug!("returning from start_vmm");
        Ok(())
    }

    /// stop_vmm stops the current VMM by sending a SIGKILL
    pub async fn stop_vmm(&mut self) -> Result<(), MachineError> {
        debug!("stop_vmm: sending sigkill to firecracker");

        // sending a SIGTERM
        // if self.cmd.is_some() && self.child_process.is_some() {
        //     let pid = self.child_process.as_ref().unwrap().id().ok_or(MachineError::Execute("stop_vmm: no pid found, maybe VMM already stopped".to_string()))?;
        //     let res = nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), nix::sys::signal::SIGTERM);
        //     if let Err(e) = res {

        //     }
        // }

        //
        if self.cmd.is_some() && self.child_process.is_some() {
            let pid = self.pid;
            self.child_process
                .as_mut()
                .unwrap()
                .kill()
                .await
                .map_err(|e| {
                    error!("vmm process already finished!");
                    MachineError::Execute(format!(
                        "firecracker process already finished, pid: {:?}",
                        pid
                    ))
                })?;
        }

        debug!("stop_vmm: no firecracker process running, not sending as signal");
        Ok(())
    }

    pub async fn do_clean_up(&mut self) -> Result<(), MachineError> {
        let mut marker = true;
        self.cleanup_once.call_once(|| {
            marker = false;
        });
        if marker {
            return Err(MachineError::Cleaning(
                "Cannot cleaning up twice".to_string(),
            ));
        }
        let clean_up_handlers = self.cleanup_funcs.to_owned();
        self.cleanup_funcs.reverse();
        clean_up_handlers.run(self).await?;

        Ok(())
    }

    /// create_machine put the machine configuration to firecracker
    /// and refresh(by get from firecracker) the machine configuration stored in `self`
    pub(super) async fn create_machine(&mut self) -> Result<(), MachineError> {
        self.agent
            .put_machine_configuration(self.cfg.machine_cfg.as_ref().unwrap())
            .await
            .map_err(|e| {
                MachineError::Initialize(format!(
                    "PutMachineConfiguration returned {}",
                    e.to_string()
                ))
            })?;
        debug!("PutMachineConfiguration returned");
        self.refresh_machine_configuration().await?;
        debug!("create_machine returning");
        Ok(())
    }

    /// refresh_machine_configuration synchronizes our cached representation of the machine configuration
    /// with that reported by the Firecracker API
    pub async fn refresh_machine_configuration(&mut self) -> Result<(), MachineError> {
        let machine_config = self.agent.get_machine_configuration().await.map_err(|e| {
            MachineError::Agent(format!(
                "Unable to inspect Firecracker MachineConfiguration. Continuing anyway. {}",
                e.to_string()
            ))
        })?;

        info!("refresh_machine_configuration: {:#?}", machine_config);
        self.machine_config = machine_config;
        Ok(())
    }

    /// Set up a signal handler to pass through to firecracker
    pub(super) async fn setup_signals(&self) -> Result<(), MachineError> {
        // judge whether forward_signals field in config exists

        debug!("Setting up signal handler: {}", todo!());

        todo!()
    }

    pub(super) async fn setup_network(&self) -> Result<(), MachineError> {
        todo!()
    }

    pub(super) async fn setup_kernel_args(&mut self) -> Result<(), MachineError> {
        let kernel_args = KernelArgs::from(self.cfg.kernel_args.as_ref().unwrap().to_owned());

        // If any network interfaces have a static IP configured, we need to set the "ip=" boot param.
        // Validation that we are not overriding an existing "ip=" setting happens in the network validation

        todo!();
        self.cfg.kernel_args = Some(kernel_args.to_string());
        todo!()
    }

    pub(super) async fn create_fifo(&self, path: &PathBuf) -> Result<(), MachineError> {
        debug!("Creating FIFO {}", path.display());
        nix::unistd::mkfifo(path, nix::sys::stat::Mode::S_IRWXU).map_err(|e| {
            error!("Failed to create log fifo: {}", e.to_string());
            MachineError::FileError(format!("Failed to create log fifo: {}", e.to_string()))
        })?;

        Ok(())
    }

    pub(super) async fn setup_logging(&self) -> Result<(), MachineError> {
        let path: &PathBuf;
        if self.cfg.log_fifo.is_some() {
            path = self.cfg.log_fifo.as_ref().unwrap();
        } else if self.cfg.log_path.is_some() {
            path = self.cfg.log_path.as_ref().unwrap();
        } else {
            info!("VMM logging disabled");
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
            info!("VMM metrics disabled");
            return Ok(());
        }
        let metrics = Metrics::default().with_metrics_path(path);
        self.agent.put_metrics(&metrics).await.map_err(|e| {
            debug!("Configured VMM metrics to {}", path.display());
            MachineError::Agent(format!("Setup metrics with agent error: {}", e.to_string()))
        })?;
        Ok(())
    }

    /// wait_for_socket waits for the given file to exist
    async fn wait_for_socket(&self, timeout_in_secs: u64) -> Result<(), MachineError> {
        if self.cfg.socket_path.is_none() {
            return Err(MachineError::FileError(
                "socket path not provided in the configuration".to_string(),
            ));
        }
        tokio::time::timeout(
            tokio::time::Duration::from_secs(timeout_in_secs),
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

    /// create_boot_source creates a boot source and configure it to microVM
    pub(super) async fn create_boot_source(
        &self,
        image_path: &PathBuf,
        initrd_path: &PathBuf,
        kernel_args: &String,
    ) -> Result<(), MachineError> {
        let bsrc = BootSource {
            kernel_image_path: image_path.to_path_buf(),
            initrd_path: Some(initrd_path.to_path_buf()),
            boot_args: Some(kernel_args.to_string()),
        };

        self.agent.put_guest_boot_source(&bsrc).await.map_err(|e| {
            info!("PutGuestBootSource: {}", e.to_string());
            MachineError::Initialize(format!("PutGuestBootSource: {}", e.to_string()))
        })?;

        debug!("PutGuestBootSource successful");
        Ok(())
    }

    /// create_network_interface creates network interface
    pub(super) async fn create_network_interface(
        &self,
        iface: NetworkInterface,
        iid: i64,
    ) -> Result<(), MachineError> {
        let iface_id = iid.to_string();

        todo!()
    }

    pub(super) async fn create_network_interfaces(&self) -> Result<(), MachineError> {
        todo!()
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

    pub(super) async fn attach_drives(&self) -> Result<(), MachineError> {
        if self.cfg.drives.is_none() {
            return Err(MachineError::Validation("drives not provided".to_string()));
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

    /// add_vsock adds a vsock to the instance
    async fn add_vsock(&self, vsock: &Vsock) -> Result<(), MachineError> {
        self.agent.put_guest_vsock(vsock).await.map_err(|e| {
            MachineError::Agent(format!("PutGuestVsock returned: {}", e.to_string()))
        })?;
        info!("attch vsock {} successful", vsock.uds_path.display());
        Ok(())
    }

    pub(super) async fn add_vsocks(&self) -> Result<(), MachineError> {
        if self.cfg.vsock_devices.is_none() {
            return Err(MachineError::Validation(
                "no vsock devices provided".to_string(),
            ));
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

    /// set_mmds_config sets the machine's mmds system
    pub(super) async fn set_mmds_config(&self, address: &Ipv4Addr) -> Result<(), MachineError> {
        let mut mmds_config = MmdsConfig::default();
        mmds_config.ipv4_address = Some(address.to_string());
        self.agent
            .put_mmds_config(&mmds_config)
            .await
            .map_err(|e| {
                error!(
                    "Setting mmds configuration failed: {}: {}",
                    address.to_string(),
                    e.to_string()
                );
                MachineError::Agent(format!(
                    "Setting mmds configuration failed: {}: {}",
                    address.to_string(),
                    e.to_string()
                ))
            })?;

        debug!("SetMmdsConfig successful");
        Ok(())
    }

    /// set_metadata sets the machine's metadata for MDDS
    pub async fn set_metadata(&self, metadata: &impl Metadata) -> Result<(), MachineError> {
        self.agent
            .put_mmds(&metadata.to_raw_string().map_err(|e| {
                error!("Setting metadata: {}", e.to_string());
                MachineError::Agent(format!("Setting metadata: {}", e.to_string()))
            })?)
            .await
            .map_err(|e| {
                error!("Setting metadata: {}", e.to_string());
                MachineError::Agent(format!("Setting metadata: {}", e.to_string()))
            })?;

        debug!("SetMetadata successful");
        Ok(())
    }

    /// update_metadata patches the machine's metadata for MDDS
    pub async fn update_matadata(&self, metadata: &impl Metadata) -> Result<(), MachineError> {
        self.agent
            .patch_mmds(&metadata.to_raw_string().map_err(|e| {
                error!(
                    "Updating metadata failed parsing parameter to string: {}",
                    e.to_string()
                );
                MachineError::Agent(format!(
                    "Updating metadata failed parsing parameter to string: {}",
                    e.to_string()
                ))
            })?)
            .await
            .map_err(|e| {
                error!("Updating metadata: {}", e.to_string());
                MachineError::Agent(format!("Updating metadata: {}", e.to_string()))
            })?;

        debug!("UpdateMetadata successful");
        Ok(())
    }

    /// get_metadata gets the machine's metadata from MDDS and unmarshals it into v
    pub async fn get_metadata<T>(&self) -> Result<T, MachineError>
    where
        T: Metadata,
    {
        let res = self.agent.get_mmds().await.map_err(|e| {
            error!("Getting metadata: {}", e.to_string());
            MachineError::Agent(format!("Getting metadata: {}", e.to_string()))
        })?;

        let res = T::from_raw_string(res).map_err(|e| {
            error!("Getting metadata failed parsing payload: {}", e.to_string());
            MachineError::Agent(format!(
                "Getting metadata failed parsing payload: {}",
                e.to_string()
            ))
        })?;

        debug!("GetMetadata successful");
        Ok(res)
    }

    /// update_guest_drive will modify the current guest drive of ID index with the new
    /// parameters of the partialDrive
    pub async fn update_guest_drive(
        &self,
        drive_id: String,
        path_on_host: String,
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
        mem_file_path: PathBuf,
        snapshot_path: PathBuf,
    ) -> Result<(), MachineError> {
        let snapshot_params = SnapshotCreateParams {
            mem_file_path: mem_file_path.to_string_lossy().to_string(),
            snapshot_path: snapshot_path.to_string_lossy().to_string(),
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

    /// create_balloon creates a balloon device if one does not exist.
    pub async fn create_balloon(
        &self,
        amount_mib: i64,
        deflate_on_oom: bool,
        stats_polling_interval_s: i64,
    ) -> Result<(), MachineError> {
        let balloon = Balloon {
            amount_mib,
            deflate_on_oom,
            stats_polling_interval_s: Some(stats_polling_interval_s),
        };

        self.agent.put_balloon(&balloon).await.map_err(|e| {
            error!("Create balloon device failed: {}", e.to_string());
            MachineError::Agent(format!("Create balloon device failed: {}", e.to_string()))
        })?;

        debug!("Created balloon device successful");
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
}
