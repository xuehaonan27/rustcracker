use std::path::PathBuf;

use procfs::process::Process;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{
    agent::agent::Agent,
    config::{HypervisorConfig, MicroVMConfig},
    firecracker::FirecrackerAsync,
    jailer::JailerAsync,
    models::*,
    raii::{Rollback, RollbackStack},
    reqres::*,
    RtckError, RtckResult,
};

type Pid = u32;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MicroVMStatus {
    None,    // no microVM running now
    Start,   // in stage of staring
    Running, // microVM running
    Paused,  // microVM paused
    Stop,    // microVM stopped
    Delete,  // microVM deleted, waiting its resources to be collected
    Failure, // microVM encountered failure
}

// #[derive(Debug)]
pub struct Hypervisor {
    // pid of the hypervisor process
    pid: Pid,

    // child of the hypervisor process
    child: tokio::process::Child,

    // process of the hypervisor process, which holds a fd to /proc/<pid>
    process: Process,

    // socket path of this hypervisor
    socket_path: PathBuf,

    // retrying times
    socket_retry: usize,

    // lock path of this hypervisor
    lock_path: PathBuf,

    // log path of this hypervisor
    log_path: PathBuf,

    // metrics path of this hypervisor
    metrics_path: PathBuf,

    // config path of this hypervisor
    config_path: Option<String>,

    // agent i.e. connection with remote
    agent: Agent,

    // instance status
    status: MicroVMStatus,
    // Some field to implement, e.g. network I/O settings...

    // clear up ?
    clear_jailer: bool,

    // jailer working directory
    jailer_working_dir: Option<PathBuf>,

    // jailer uid and gid
    uid_gid: Option<(u32, u32)>, // (uid, gid)

    // mounting points
    // mounts: Vec<PathBuf>,

    // rollback stack
    rollbacks: RollbackStack,
}

impl Hypervisor {
    /// Create a hypervisor
    pub async fn new(config: &HypervisorConfig) -> RtckResult<Self> {
        config.validate()?;

        // if config.frck_export_path.is_some() {
        //     config.export_config_async().await?;
        // }

        let (
            pid,
            stream,
            firecracker,
            child,
            process,
            clear_jailer,
            jailer_working_dir,
            uid_gid,
            mut rollbacks,
        ) = if let Some(true) = config.using_jailer {
            let mut rollbacks = RollbackStack::new();
            let mut jailer = JailerAsync::from_config(&config)?;

            // jail the firecracker
            let instance_dir = jailer.jail().await?;
            rollbacks.push(Rollback::Jailing { instance_dir });

            // spawn the firecracker process
            // error: fail to launch the process
            let child = jailer.launch().await?;
            let pid = child
                .id()
                .ok_or(RtckError::Hypervisor("child fail to get pid".to_string()))?;
            rollbacks.push(Rollback::StopProcess { pid });

            // wait socket
            // error: socket do not exists after <timeout> secs
            jailer
                .waiting_socket(tokio::time::Duration::from_secs(config.launch_timeout))
                .await?;
            rollbacks.insert_1(Rollback::RemoveSocket {
                // unwrap safe because correct arguments provided
                // otherwise rollback would occur right after `jail`
                path: jailer.get_socket_path_exported().cloned().unwrap(),
            });

            // error: the process doesn't exist, or if you don't have permission to access it.
            // it's recommended that rustcracker run as root.
            let process = Process::new(pid as i32)
                .map_err(|_| RtckError::Hypervisor("child fail to get process".to_string()))?;

            let stream = jailer.connect(config.socket_retry).await?;

            let jailer_working_dir = jailer.get_jailer_workspace_dir().cloned();

            let uid = jailer.get_uid();

            let gid = jailer.get_gid();

            let firecracker = FirecrackerAsync::from_jailer(jailer)?;

            let clear_jailer = if config.clear_jailer.is_none() || !config.clear_jailer.unwrap() {
                false
            } else {
                true
            };

            (
                pid,
                stream,
                firecracker,
                child,
                process,
                clear_jailer,
                jailer_working_dir,
                Some((uid, gid)),
                rollbacks,
            )
        } else {
            let mut rollbacks = RollbackStack::new();
            let firecracker = FirecrackerAsync::from_config(&config)?;

            // spawn the firecracker process
            // error: fail to launch the process
            let child = firecracker.launch().await?;
            let pid = child
                .id()
                .ok_or(RtckError::Hypervisor("child fail to get pid".to_string()))?;
            rollbacks.push(Rollback::StopProcess { pid });

            // wait socket
            // error: socket do not exists after <timeout> secs
            firecracker
                .waiting_socket(tokio::time::Duration::from_secs(config.launch_timeout))
                .await?;
            rollbacks.insert_1(Rollback::RemoveSocket {
                path: firecracker.get_socket_path(),
            });

            // error: the process doesn't exist, or if you don't have permission to access it.
            // it's recommended that rustcracker run as root.
            let process = Process::new(pid as i32)
                .map_err(|_| RtckError::Hypervisor("child fail to get process".to_string()))?;

            let stream = firecracker.connect(config.socket_retry).await?;

            (
                pid,
                stream,
                firecracker,
                child,
                process,
                false,
                None,
                None,
                rollbacks,
            )
        };

        let lock = fslock::LockFile::open(&firecracker.lock_path)
            .map_err(|_| RtckError::Hypervisor("creating file lock".to_string()))?;
        rollbacks.insert_1(Rollback::RemoveFsLock {
            path: firecracker.lock_path.clone(),
        });

        let agent = Agent::from_stream_lock(stream, lock);

        Ok(Self {
            pid,
            child,
            process,
            socket_path: firecracker.socket,
            socket_retry: config.socket_retry,
            lock_path: firecracker.lock_path,
            log_path: firecracker.log_path,
            metrics_path: firecracker.metrics_path,
            config_path: firecracker.config_path,
            agent,
            status: MicroVMStatus::None,
            clear_jailer,
            jailer_working_dir,
            uid_gid,
            // mounts: Vec::new(),
            rollbacks,
        })
    }

    /// Ping firecracker to check its soundness
    pub async fn ping_remote(&mut self) -> RtckResult<()> {
        let event = GetFirecrackerVersion::new();
        let _ = self.agent.event(event).await?;
        Ok(())
    }

    /// Check hypervisor's state
    pub async fn check_state(&mut self) -> RtckResult<()> {
        if self.status == MicroVMStatus::None {
            log::warn!("no microVM running");
            return Ok(());
        }
        // check if the process is still running
        if !self.process.is_alive() {
            // process is no longer running, fetch output and error
            if let Some(_status) = self
                .child
                .try_wait()
                .map_err(|_| RtckError::Hypervisor("fail to wait the process".to_string()))?
            {
                self.fetch_output().await?;
            } else {
                log::warn!("Process has terminated, but no status code available");
            }
        }
        Ok(())
    }

    async fn fetch_output(&mut self) -> RtckResult<()> {
        let mut stdout = self
            .child
            .stdout
            .take()
            .ok_or(RtckError::Hypervisor("fail to take stdout".to_string()))?;

        let mut stderr = self
            .child
            .stderr
            .take()
            .ok_or(RtckError::Hypervisor("fail to take stdout".to_string()))?;

        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();

        // Read stdout and stderr concurrently
        let stdout_future = async {
            let _ = stdout
                .read_to_end(&mut stdout_buf)
                .await
                .map_err(|_| RtckError::Hypervisor("fail to read stdout from child".to_string()));
        };

        let stderr_future = async {
            let _ = stderr
                .read_to_end(&mut stderr_buf)
                .await
                .map_err(|_| RtckError::Hypervisor("fail to read stderr from child".to_string()));
        };

        tokio::join!(stdout_future, stderr_future);

        // Open the log file in append mode
        let mut log_file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .await
            .map_err(|_| RtckError::Hypervisor("fail to open log file".to_string()))?;

        // Write stdout and stderr to the log file
        log_file
            .write_all(&stdout_buf)
            .await
            .map_err(|_| RtckError::Hypervisor("fail to write stdout to log".to_string()))?;
        log_file
            .write_all(&stderr_buf)
            .await
            .map_err(|_| RtckError::Hypervisor("fail to write stderr to log".to_string()))?;

        Ok(())
    }

    /// Automatically configure the machine.
    /// User must guarantee that `config` passed to the machine contains
    /// valid firecracker configuration (`frck_config`).
    async fn configure(&mut self, config: &MicroVMConfig) -> RtckResult<()> {
        // // If configuration has been exported, then the machine should have been configured.
        // if config.frck_export_path.is_some() {
        //     return Ok(());
        // }

        // // User must guarantee that proper
        // let frck_config = self
        //     .config
        //     .frck_config
        //     .as_ref()
        //     .ok_or(RtckError::Config("no firecracker config".to_string()))?;

        // Logger
        {
            if let Some(logger) = &config.logger {
                let put_logger = PutLogger::new(logger.clone());
                let res = self.agent.event(put_logger).await.map_err(|e| {
                    log::error!("PutLogger event failed");
                    e
                })?;
                if res.is_err() {
                    log::error!("PutLogger failed");
                }
            }
        }

        // Metrics
        {
            if let Some(metrics) = &config.metrics {
                let put_metrics = PutMetrics::new(metrics.clone());
                let res = self.agent.event(put_metrics).await.map_err(|e| {
                    log::error!("PutMetrics event failed");
                    e
                })?;
                if res.is_err() {
                    log::error!("PutMetrics failed");
                }
            }
        }

        // Guest boot source
        {
            let boot_source = if let Some(jailer_working_dir) = &self.jailer_working_dir {
                // using jailer
                // jailer_working_dir = <chroot_base>/<exec_file_name>/<id>/root
                if let Some(boot_source) = &config.boot_source {
                    // mount the kernel directory
                    let target_dir = jailer_working_dir.join("kernel");
                    tokio::fs::create_dir_all(&target_dir).await.map_err(|_| {
                        RtckError::Hypervisor("fail to create kernel dir".to_string())
                    })?;
                    let source = PathBuf::from(&boot_source.kernel_image_path)
                        .canonicalize()
                        .map_err(|_| RtckError::Config("invalid path".to_string()))?;
                    let source_dir = source
                        .parent()
                        .ok_or(RtckError::Config("invalid path".to_string()))?;
                    let kernel_file = source.file_name().ok_or(RtckError::Config(
                        "invalid kernel image file path".to_string(),
                    ))?;

                    use nix::mount::{mount, MsFlags};
                    match mount(
                        Some(source_dir),
                        &target_dir,
                        None::<&PathBuf>,
                        MsFlags::MS_BIND,
                        None::<&PathBuf>,
                    ) {
                        Ok(_) => {
                            self.rollbacks.insert_1(Rollback::Umount {
                                mount_point: target_dir,
                            });
                            // self.mounts.push(target_dir);
                        }
                        Err(e) => {
                            return Err(RtckError::Hypervisor(format!(
                                "fail to mount kernel image dir into jailer, errno = {}",
                                e
                            )))
                        }
                    }
                    let mut boot_source = config.boot_source.clone().unwrap();
                    let mut jailed_kernel_image_path = PathBuf::from("/kernel");
                    jailed_kernel_image_path.push(kernel_file);
                    boot_source.kernel_image_path =
                        jailed_kernel_image_path.to_string_lossy().to_string();
                    Some(boot_source)
                } else {
                    None
                }
            } else {
                config.boot_source.clone()
            };

            if let Some(boot_source) = &boot_source {
                // if let Some(boot_source) = &config.boot_source {
                let put_guest_boot_source = PutGuestBootSource::new(boot_source.clone());
                let res = self.agent.event(put_guest_boot_source).await.map_err(|e| {
                    log::error!("PutGuestBootSource event failed");
                    e
                })?;
                if res.is_err() {
                    log::error!("PutGuestBootSource failed");
                }
            }
        }

        // Guest drives
        {
            if let Some(drives) = &config.drives {
                for drive in drives {
                    let drive = if let Some(jailer_working_dir) = &self.jailer_working_dir {
                        // using jailer
                        // jailer_working_dir = <chroot_base>/<exec_file_name>/<id>/root
                        // mount the drives directory
                        // TODO: mount the drive socket?
                        let target_dir =
                            jailer_working_dir.join(format!("drives{}", drive.drive_id));
                        tokio::fs::create_dir(&target_dir).await.map_err(|_| {
                            RtckError::Hypervisor(format!(
                                "fail to create dir for drives {}",
                                drive.drive_id
                            ))
                        })?;
                        let source = PathBuf::from(&drive.path_on_host)
                            .canonicalize()
                            .map_err(|_| RtckError::Config("invalid path".to_string()))?;
                        let source_dir = source
                            .parent()
                            .ok_or(RtckError::Config("invalid path".to_string()))?;
                        let drive_file = source
                            .file_name()
                            .ok_or(RtckError::Config("invalid drive file path".to_string()))?;

                        use nix::mount::{mount, MsFlags};
                        match mount(
                            Some(source_dir),
                            &target_dir,
                            None::<&PathBuf>,
                            MsFlags::MS_BIND,
                            None::<&PathBuf>,
                        ) {
                            Ok(_) => {
                                self.rollbacks.insert_1(Rollback::Umount {
                                    mount_point: target_dir,
                                });
                                // self.mounts.push(target_dir);
                            }
                            Err(e) => {
                                return Err(RtckError::Hypervisor(format!(
                                    "fail to mount drive {} dir into jailer, errno = {}",
                                    drive.drive_id, e
                                )))
                            }
                        }

                        use nix::unistd::{Gid, Uid};
                        // change the owner of the drive
                        let (uid, gid) = self.uid_gid.ok_or(RtckError::Hypervisor(
                            "no uid and gid found in jailer".to_string(),
                        ))?;
                        let metadata = std::fs::metadata(&source).map_err(|_| {
                            RtckError::Hypervisor(format!(
                                "fail to get metadata of source path {:?}",
                                &source
                            ))
                        })?;
                        use std::os::unix::fs::MetadataExt;
                        let original_uid = metadata.uid();
                        let original_gid = metadata.gid();
                        nix::unistd::chown(
                            &source,
                            Some(Uid::from_raw(uid)),
                            Some(Gid::from_raw(gid)),
                        )
                        .map_err(|_| {
                            RtckError::Hypervisor("fail to change the owner of jailer".to_string())
                        })?;
                        self.rollbacks.insert_1(Rollback::Chown {
                            path: source.clone(),
                            original_uid,
                            original_gid,
                        });

                        let mut drive = drive.clone();
                        let mut jailed_drive_file_path =
                            PathBuf::from(format!("/drives{}", drive.drive_id));
                        jailed_drive_file_path.push(drive_file);
                        drive.path_on_host = jailed_drive_file_path.to_string_lossy().to_string();
                        drive
                    } else {
                        drive.clone()
                    };

                    let put_guest_drive_by_id = PutGuestDriveByID::new(drive.clone());
                    let res = self.agent.event(put_guest_drive_by_id).await.map_err(|e| {
                        log::error!("PutGuestDriveByIDResponse event failed");
                        e
                    })?;
                    if res.is_err() {
                        log::error!("PutGuestDriveById failed");
                    }
                }
            }
        }

        // Guest network interfaces
        {
            if let Some(ifaces) = &config.network_interfaces {
                for iface in ifaces {
                    let put_guest_network_interface_by_id =
                        PutGuestNetworkInterfaceByID::new(iface.clone());
                    let res = self
                        .agent
                        .event(put_guest_network_interface_by_id)
                        .await
                        .map_err(|e| {
                            log::error!("PutGuestNetworkInterfaceByID event failed");
                            e
                        })?;
                    if res.is_err() {
                        log::error!("PutGuestNetworkInterfaceById failed");
                    }
                }
            }
        }

        // Vsocks
        {
            if let Some(vsocks) = &config.vsock_devices {
                for vsock in vsocks {
                    let put_guest_vsock = PutGuestVsock::new(vsock.clone());
                    let res = self.agent.event(put_guest_vsock).await.map_err(|e| {
                        log::error!("PutGuestVsock event failed");
                        e
                    })?;
                    if res.is_err() {
                        log::error!("PutGuestVsock failed");
                    }
                }
            }
        }

        // CPU configuration
        {
            if let Some(cpu_config) = &config.cpu_config {
                let put_cpu_configuration = PutCpuConfiguration::new(cpu_config.clone());
                let res = self.agent.event(put_cpu_configuration).await.map_err(|e| {
                    log::error!("PutCpuConfiguration event failed");
                    e
                })?;
                if res.is_err() {
                    log::error!("PutCpuConfiguration failed");
                }
            }
        }

        // Machine configuration
        {
            if let Some(machine_config) = &config.machine_config {
                let put_machine_configuration =
                    PutMachineConfiguration::new(machine_config.clone());
                let res = self
                    .agent
                    .event(put_machine_configuration)
                    .await
                    .map_err(|e| {
                        log::error!("PutMachineConfiguration event failed");
                        e
                    })?;
                if res.is_err() {
                    log::error!("PutMachineConfiguration failed");
                }
            }
        }

        // Balloon
        {
            if let Some(balloon) = &config.balloon {
                let put_balloon = PutBalloon::new(balloon.clone());
                let res = self.agent.event(put_balloon).await.map_err(|e| {
                    log::error!("PutBalloon event failed");
                    e
                })?;
                if res.is_err() {
                    log::error!("PutBalloon failed");
                }
            }
        }

        // Entropy device
        {
            if let Some(entropy_device) = &config.entropy_device {
                let put_entropy = PutEntropy::new(entropy_device.clone());
                let res = self.agent.event(put_entropy).await.map_err(|e| {
                    log::error!("PutEntropy event failed");
                    e
                })?;
                if res.is_err() {
                    log::error!("PutEntropy failed");
                }
            }
        }

        // Initial mmds content
        {
            if let Some(content) = &config.init_metadata {
                let put_mmds = PutMmds::new(content.clone());
                let res = self.agent.event(put_mmds).await.map_err(|e| {
                    log::error!("PutMmds event failed");
                    e
                })?;
                if res.is_err() {
                    log::error!("PutMmds failed");
                }
            }
        }

        Ok(())
    }

    /// Run a microVM instance by passing a microVM configuraion
    pub async fn start(&mut self, config: &MicroVMConfig) -> RtckResult<()> {
        // change microVM status in hypervisor
        self.status = MicroVMStatus::Start;

        self.configure(config).await.map_err(|_| {
            log::error!("start fail, {} {}", file!(), line!());
            // change microVM status in hypervisor
            self.status = MicroVMStatus::Failure;
            RtckError::Hypervisor("fail to start".to_string())
        })?;

        let start_machine = CreateSyncAction::new(InstanceActionInfo {
            action_type: ActionType::InstanceStart,
        });

        let res = self.agent.event(start_machine).await.map_err(|_| {
            {
                log::error!("start fail, {} {}", file!(), line!());
                // change microVM status in hypervisor
                self.status = MicroVMStatus::Failure;
                RtckError::Hypervisor("fail to start".to_string())
            }
        })?;

        if res.is_err() {
            log::error!("start fail, {} {}", file!(), line!());
            // change microVM status in hypervisor
            self.status = MicroVMStatus::Failure;
            Err(RtckError::Hypervisor("fail to start".to_string()))
        } else {
            // change microVM status in hypervisor
            self.status = MicroVMStatus::Running;
            Ok(())
        }
    }

    /// Pause the machine by notifying the hypervisor
    pub async fn pause(&mut self) -> RtckResult<()> {
        if self.status != MicroVMStatus::Running {
            log::warn!("can not pause a microVM that's not running");
            return Ok(());
        }

        let pause_machine = PatchVm::new(vm::VM_STATE_PAUSED);

        let res = self.agent.event(pause_machine).await.map_err(|_| {
            log::error!("pause fail");
            // no need for changing status here since pausing failure isn't a fatal error
            RtckError::Hypervisor("fail to pause".to_string())
        })?;

        if res.is_err() {
            log::error!("pause fail");
            Err(RtckError::Hypervisor("fail to pause".to_string()))
        } else {
            self.status = MicroVMStatus::Paused;
            Ok(())
        }
    }

    /// Resume the machine by notifying the hypervisor
    pub async fn resume(&mut self) -> RtckResult<()> {
        if self.status != MicroVMStatus::Paused {
            log::warn!("can not resume a microVM that's not paused");
            return Ok(());
        }

        let resume_machine = PatchVm::new(vm::VM_STATE_RESUMED);

        let res = self.agent.event(resume_machine).await.map_err(|_| {
            log::error!("resume fail");
            RtckError::Machine("fail to resume".to_string())
        })?;

        if res.is_err() {
            log::error!("resume fail");
            Err(RtckError::Machine("fail to resume".to_string()))
        } else {
            self.status = MicroVMStatus::Running;
            Ok(())
        }
    }

    /// Stop the machine by notifying the hypervisor.
    /// Hypervisor should still be valid.
    pub async fn stop(&mut self) -> RtckResult<()> {
        let stop_machine = CreateSyncAction::new(InstanceActionInfo {
            action_type: ActionType::SendCtrlAtlDel,
        });

        let res = self.agent.event(stop_machine).await.map_err(|_| {
            log::error!("stop fail");
            RtckError::Machine("fail to stop".to_string())
        })?;

        if res.is_err() {
            log::error!("stop fail");
            Err(RtckError::Hypervisor("fail to stop".to_string()))
        } else {
            self.status = MicroVMStatus::Stop;
            Ok(())
        }
    }

    /// Terminate the hypervisor by sending SIGTERM
    /// Note: this command will terminate firecracker itself.
    #[cfg(any(target_os = "linux", target_os = "unix"))]
    async fn terminate(&mut self) -> RtckResult<()> {
        use nix::{
            sys::signal::{kill, Signal},
            unistd::Pid,
        };
        // the hypervisor occupies the pid by opening fd to it (procfs).
        // so kill -9 to this pid is safe.
        kill(Pid::from_raw(self.pid as i32), Signal::SIGTERM)
            .map_err(|_| RtckError::Machine("fail to terminate".to_string()))
    }

    /// Terminate the hypervisor by sending SIGKILL
    /// Note: this command will kill firecracker itself.
    #[cfg(any(target_os = "linux", target_os = "unix"))]
    async fn kill(&mut self) -> RtckResult<()> {
        // the hypervisor occupies the pid by opening fd to it (procfs).
        // so kill -9 to this pid is safe.
        use nix::sys::signal::{kill, Signal};
        kill(nix::unistd::Pid::from_raw(self.pid as i32), Signal::SIGKILL)
            // kill -9 should not trigger this error since SIGKILL is not blockable
            .map_err(|_| RtckError::Machine("fail to terminate".to_string()))
    }

    /// Create a snapshot
    pub async fn snapshot<P: AsRef<str>, Q: AsRef<str>>(
        &mut self,
        state_path: P,
        mem_path: Q,
        _type: SnapshotType,
    ) -> RtckResult<()> {
        let create_snapshot = CreateSnapshot::new(SnapshotCreateParams {
            mem_file_path: state_path.as_ref().to_string(),
            snapshot_path: mem_path.as_ref().to_string(),
            snapshot_type: Some(_type),
            version: None,
        });

        let res = self.agent.event(create_snapshot).await?;
        if res.is_err() {
            log::error!("Machine::snapshot fail");
            return Err(RtckError::Machine("fail to create snapshot".to_string()));
        }
        Ok(())
    }

    /// Delete the machine by notifying firecracker
    pub async fn delete(mut self) -> RtckResult<()> {
        let _ = self.stop().await;
        drop(self);

        // delete network TUN/TAP interface

        // delete cgroups (if exists)

        Ok(())
    }

    /// Check microVM status
    pub async fn poll_status(&self) -> MicroVMStatus {
        todo!()
    }
}
