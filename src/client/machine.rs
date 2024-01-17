use std::{net::Ipv4Addr, path::PathBuf};

use async_trait::async_trait;

use log::{debug, error, info, trace, warn};

use crate::{
    model::{
        self,
        balloon::Balloon,
        balloon_stats::BalloonStatistics,
        balloon_stats_update::BalloonStatsUpdate,
        balloon_update::BalloonUpdate,
        drive::Drive,
        instance_action_info::InstanceActionInfo,
        instance_info::InstanceInfo,
        machine_configuration::MachineConfiguration,
        mmds_config::MmdsConfig,
        partial_drive::PartialDrive,
        partial_network_interface::PartialNetworkInterface,
        rate_limiter::RateLimiterSet,
        snapshot_create_params::SnapshotCreateParams,
        vm::{VM_STATE_PAUSED, VM_STATE_RESUMED},
        vsock::Vsock, network_interface::NetworkInterface, boot_source::BootSource,
    },
    utils::{Json, Metadata},
};

use super::{
    agent::{Agent, AgentError},
    handlers::Handlers,
    jailer::{JailerConfig, StdioTypes},
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

// Config is a collection of user-configurable VMM settings
// #[derive(Deserialize, Serialize, Debug, Clone)]
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
    pub log_level: Option<model::logger::LogLevel>,

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
    pub drives: Option<Vec<model::drive::Drive>>,

    // NetworkInterfaces specifies the tap devices that should be made available
    // to the microVM.
    pub network_interfaces: Option<Vec<model::network_interface::NetworkInterface>>,

    // FifoLogWriter is an io.Writer(Stdio) that is used to redirect the contents of the
    // fifo log to the writer.
    // pub(crate) fifo_log_writer: Option<std::process::Stdio>,
    pub fifo_log_writer: Option<StdioTypes>,

    // VsockDevices specifies the vsock devices that should be made available to
    // the microVM.
    pub vsock_devices: Option<Vec<model::vsock::Vsock>>,

    // MachineCfg represents the firecracker microVM process configuration
    pub machine_cfg: Option<model::machine_configuration::MachineConfiguration>,

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
    // forward_signals: Vec<>,

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
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), MachineError> {
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

pub struct Machine {
    pub(crate) handlers: Handlers,

    pub(crate) cfg: Config,

    agent: Agent,
    pub(crate) cmd: std::process::Command,
    logger: crate::model::logger::Logger,

    // The actual machine config as reported by Firecracker
    // id est, not the config set by user, which should be a field of `cfg`
    machine_config: MachineConfiguration,

    // startOnce ensures that the machine can only be started once
    start_once: std::sync::Once,

    // exitCh is a channel which gets closed when the VMM exits
    exit_ch: (),

    // fatalErr records an error that either stops or prevent starting the VMM
    fatalerr: Option<MachineError>,

    // callbacks that should be run when the machine is being torn down
    cleanup_once: std::sync::Once,

    cleanup_funcs: Vec<Option<Box<dyn FnOnce() -> Result<(), MachineError>>>>,
}

#[derive(thiserror::Error, Debug)]
pub enum MachineError {
    /// Mostly problems related to directories error or unavailable files
    #[error("Could not set up environment(e.g. file, linking) the machine, reason: {0}")]
    Setup(String),
    /// Failure when validating the configuration before starting the microVM
    #[error("Invalid configuration for the machine, reason: {0}")]
    Validation(String),
    /// Related to communication with the socket to configure the microVM which failed
    #[error("Could not put initial configuration for the machine, reason: {0}")]
    Configure(String),
    /// The process didn't start properly or an error occurred while trying to run it
    #[error("Fail to start or run the machine, reason: {0}")]
    Execute(String),
    /// Failure when cleaning up the machine
    #[error("Could not clean up the machine properly, reason: {0}")]
    Cleaning(String),
    #[error("Agent could not communicate with firecracker process, reason: {0}")]
    Agent(String),
    #[error("Could not dump the file, reason: {0}")]
    Dump(String),
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

impl Machine {
    /// new initializes a new Machine instance and performs validation of the
    /// provided Config.
    pub fn new(mut cfg: Config) -> Result<Machine, MachineError> {
        // create a channel for communicate with microVM

        // set vmid for microVM
        if cfg.vmid.is_none() {
            let random_id = uuid::Uuid::new_v4().to_string();
            cfg.vmid = Some(random_id);
        }

        // set default handlers for microVM
        // let mut m_handlers = DEFAULT_HANDLERS;

        // if cfg.jailer_cfg.is_some() {
        //     // m_handlers.validation.push(JailerConfigValidationHandler);

        // }
        todo!()
    }

    pub async fn start_instance(&self) -> Result<(), MachineError> {
        debug!("called Machine::start");
        self.agent
            .create_sync_action(InstanceActionInfo::instance_start())
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
            .create_sync_action(InstanceActionInfo::send_ctrl_alt_del())
            .await
            .map_err(|e| MachineError::Execute(e.to_string()))?;
        Ok(())
    }

    pub async fn pause(&self) -> Result<(), MachineError> {
        debug!("called Machine::pause");
        self.agent
            .patch_vm(VM_STATE_PAUSED)
            .await
            .map_err(|e| MachineError::Execute(e.to_string()))?;
        Ok(())
    }

    pub async fn resume(&self) -> Result<(), MachineError> {
        debug!("called Machine::resume");
        self.agent
            .patch_vm(VM_STATE_RESUMED)
            .await
            .map_err(|e| MachineError::Execute(e.to_string()))?;
        Ok(())
    }

    // pub async fn do_clean_up(&mut self) -> Result<(), MachineError> {
    //     self.cleanup_once.call_once(|| {
    //         self.cleanup_funcs.reverse();
    //         self.cleanup_funcs.iter_mut().for_each(|f| {
    //             let f = f.take();
    //             if f.is_some() {

    //             }
    //         });
    //     });
    //     Ok(())
    // }

    /// create_machine put the machine configuration to firecracker
    /// and refresh(by get from firecracker) the machine configuration stored in `self`
    pub async fn create_machine(&mut self) -> Result<(), MachineError> {
        self.agent
            .put_machine_configuration(self.cfg.machine_cfg.as_ref().unwrap())
            .await
            .map_err(|e| {
                MachineError::Configure(format!(
                    "PutMachineConfiguration returned {}",
                    e.to_string()
                ))
            })?;
        debug!("PutMachineConfiguration returned");
        self.refresh_machine_configuration().await?;
        // "Unable to inspect Firecracker MachineConfiguration. Continuing anyway. %s"
        debug!("create_machine returning");
        Ok(())
    }

    /// refresh_machine_configuration synchronizes our cached representation of the machine configuration
    /// with that reported by the Firecracker API
    pub async fn refresh_machine_configuration(&mut self) -> Result<(), MachineError> {
        let machine_config = self.agent.get_machine_configuration().await.map_err(|e| {
            MachineError::Dump(format!(
                "Unable to inspect Firecracker MachineConfiguration. Continuing anyway. {}",
                e.to_string()
            ))
        })?;

        info!("refresh_machine_configuration: {:#?}", machine_config);
        self.machine_config = machine_config;
        Ok(())
    }

    /// Set up a signal handler to pass through to firecracker
    pub async fn setup_signals(&self) -> Result<(), MachineError> {
        // judge whether forward_signals field in config exists

        debug!("Setting up signal handler: {}", todo!());

        todo!()
    }

    /// wait_for_socket waits for the given file to exist
    pub async fn wait_for_socket(&self, timeout_in_secs: u64) -> Result<(), MachineError> {
        if self.cfg.socket_path.is_none() {
            return Err(MachineError::Setup(
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
            MachineError::Setup(format!(
                "firecracker fail to create socket at the given path after {} seconds",
                timeout_in_secs
            ))
        })?;

        Ok(())
    }

    /// create_boot_source creates a boot source and configure it to microVM
    pub async fn create_boot_source(&self, image_path: PathBuf, initrd_path: PathBuf, kernel_args: String) -> Result<(), MachineError> {
        let bsrc = BootSource {
            kernel_image_path: image_path,
            initrd_path: Some(initrd_path),
            boot_args: Some(kernel_args),
        };

        self.agent.put_guest_boot_source(bsrc).await.map_err(|e| {
            info!("PutGuestBootSource: {}", e.to_string());
            MachineError::Configure(format!("PutGuestBootSource: {}", e.to_string()))
        })?;

        debug!("PutGuestBootSource successful");
        Ok(())
    }

    /// create_network_interface creates network interface
    pub async fn create_network_interface(&self, iface: NetworkInterface, iid: i64) -> Result<(), MachineError> {
        let iface_id = iid.to_string();
        
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
            .patch_guest_network_interface_by_id(iface)
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
    pub async fn attach_drive(&self, dev: Drive) -> Result<(), MachineError> {
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
            MachineError::Configure(format!(
                "Attach drive failed: {}: {}",
                host_path.display(),
                e.to_string()
            ))
        })?;

        info!("Attached drive {}", host_path.display());
        Ok(())
    }

    /// add_vsock adds a vsock to the instance
    pub async fn add_vsock(&mut self, vsock: &Vsock) -> Result<(), MachineError> {
        self.agent.put_guest_vsock(vsock).await.map_err(|e| {
            MachineError::Configure(format!("PutGuestVsock returned: {}", e.to_string()))
        })?;
        info!("attch vsock {} successful", vsock.uds_path);
        Ok(())
    }

    /// set_mmds_config sets the machine's mmds system
    pub async fn set_mmds_config(&self, address: Ipv4Addr) -> Result<(), MachineError> {
        let mut mmds_config = MmdsConfig::default();
        mmds_config.ipv4_address = Some(address.to_string());
        self.agent.put_mmds_config(mmds_config).await.map_err(|e| {
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
    pub async fn set_metadata(&self, metadata: impl Metadata) -> Result<(), MachineError> {
        self.agent
            .put_mmds(metadata.to_raw_string().map_err(|e| {
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
    pub async fn update_matadata(&self, metadata: impl Metadata) -> Result<(), MachineError> {
        self.agent
            .patch_mmds(metadata.to_raw_string().map_err(|e| {
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
            .patch_guest_drive_by_id(partial_drive)
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
            .create_snapshot(snapshot_params)
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

        self.agent.put_balloon(balloon).await.map_err(|e| {
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
            .patch_balloon(balloon_update)
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
            .patch_balloon_stats_interval(balloon_stats_update)
            .await
            .map_err(|e| {
                error!("UpdateBalloonStats failed: {}", e.to_string());
                MachineError::Agent(format!("UpdateBalloonStats failed: {}", e.to_string()))
            })?;

        debug!("UpdateBalloonStats successful");
        Ok(())
    }
}
