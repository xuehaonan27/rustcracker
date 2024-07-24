use std::{path::PathBuf, process::ExitStatus};

use crate::{
    agent::sync_agent::Agent,
    config::{HypervisorConfig, MicroVMConfig},
    firecracker::Firecracker,
    hplog::{HPLogger, LogTo},
    hypervisor::MicroVMStatus,
    jailer::Jailer,
    models::*,
    raii::{Rollback, RollbackStack},
    reqres::*,
    RtckError, RtckResult,
};

use log::Level::*;

pub struct Hypervisor {
    // instance id of the hypervisor process
    id: String,

    // pid of the hypervisor process
    pid: u32,

    // child of the hypervisor process
    child: std::process::Child,

    // process of the hypervisor process, which holds a fd to /proc/<pid>
    // process: Process,

    // socket path of this hypervisor
    socket_path: PathBuf,

    // retrying times
    socket_retry: usize,

    // lock path of this hypervisor
    lock_path: PathBuf,

    // log path of this hypervisor
    log_path: Option<PathBuf>,

    // hypervisor logger
    logger: HPLogger,

    // metrics path of this hypervisor
    // metrics_path: PathBuf,

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

    // intervals in seconds for polling the status of microVM
    // when waiting for the user to give up the microVM
    poll_status_secs: u64,
}

impl Hypervisor {
    pub fn new(config: &HypervisorConfig) -> RtckResult<Self> {
        config.validate()?;

        let (
            pid,
            stream,
            firecracker,
            child,
            // process,
            clear_jailer,
            jailer_working_dir,
            uid_gid,
            mut rollbacks,
        ) = if let Some(true) = config.using_jailer {
            let mut rollbacks = RollbackStack::new();
            let mut jailer = Jailer::from_config(&config)?;

            let clear_jailer = if config.clear_jailer.is_none() || !config.clear_jailer.unwrap() {
                false
            } else {
                true
            };

            // jail the firecracker
            let instance_dir = jailer.jail()?;
            if let Some(true) = config.using_jailer {
                rollbacks.push(Rollback::Jailing {
                    clear: clear_jailer,
                    instance_dir,
                });
            }

            // spawn the firecracker process
            // error: fail to launch the process
            let child = jailer.launch()?;
            let pid = child.id();
            rollbacks.push(Rollback::StopProcess { pid });

            // wait socket
            // error: socket do not exists after <timeout> secs
            jailer.waiting_socket(std::time::Duration::from_secs(config.launch_timeout))?;
            rollbacks.insert_1(Rollback::RemoveSocket {
                // unwrap safe because correct arguments provided
                // otherwise rollback would occur right after `jail`
                path: jailer.get_socket_path_exported().cloned().unwrap(),
            });

            // error: the process doesn't exist, or if you don't have permission to access it.
            // it's recommended that rustcracker run as root.
            // let process = Process::new(pid as i32)
            //     .map_err(|_| RtckError::Hypervisor("child fail to get process".to_string()))?;

            let stream = jailer.connect(config.socket_retry)?;

            let jailer_working_dir = jailer.get_jailer_workspace_dir().cloned();

            let uid = jailer.get_uid();

            let gid = jailer.get_gid();

            let firecracker = Firecracker::from_jailer(jailer)?;

            (
                pid,
                stream,
                firecracker,
                child,
                // process,
                clear_jailer,
                jailer_working_dir,
                Some((uid, gid)),
                rollbacks,
            )
        } else {
            let mut rollbacks = RollbackStack::new();
            let firecracker = Firecracker::from_config(&config)?;

            // spawn the firecracker process
            // error: fail to launch the process
            let child = firecracker.launch()?;
            let pid = child.id();
            rollbacks.push(Rollback::StopProcess { pid });

            // wait socket
            // error: socket do not exists after <timeout> secs
            firecracker.waiting_socket(std::time::Duration::from_secs(config.launch_timeout))?;
            rollbacks.insert_1(Rollback::RemoveSocket {
                path: firecracker.get_socket_path(),
            });

            // error: the process doesn't exist, or if you don't have permission to access it.
            // it's recommended that rustcracker run as root.
            // let process = Process::new(pid as i32)
            //     .map_err(|_| RtckError::Hypervisor("child fail to get process".to_string()))?;

            let stream = firecracker.connect(config.socket_retry)?;

            (
                pid,
                stream,
                firecracker,
                child,
                // process,
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

        stream.set_nonblocking(true).map_err(|_| {
            RtckError::Hypervisor("couldn't set stream to non-blocking".to_string())
        })?;
        let agent = Agent::from_stream_lock(stream, lock);

        let logger = HPLogger::new(
            config.id.clone(),
            match firecracker.log_path {
                None => LogTo::Stdout,
                Some(ref s) => LogTo::File(s.clone()),
            },
        )?;

        Ok(Self {
            id: firecracker.id,
            pid,
            child,
            // process,
            socket_path: firecracker.socket,
            socket_retry: config.socket_retry,
            lock_path: firecracker.lock_path,
            log_path: firecracker.log_path,
            logger,
            // metrics_path: firecracker.metrics_path,
            config_path: firecracker.config_path,
            agent,
            status: MicroVMStatus::None,
            clear_jailer,
            jailer_working_dir,
            uid_gid,
            // mounts: Vec::new(),
            rollbacks,
            poll_status_secs: config.poll_status_secs,
        })
    }

    /// Log
    pub fn log(&self, level: log::Level, msg: &str) {
        self.logger.log(level, msg);
    }

    /// Ping firecracker to check its soundness
    pub fn ping_remote(&mut self) -> RtckResult<()> {
        let event = GetFirecrackerVersion::new();
        let _ = self.agent.event(event)?;
        Ok(())
    }

    /// Logger configuration. Nothing to rollback in this step.
    fn logger_configure(&mut self, config: &MicroVMConfig) -> RtckResult<()> {
        if let Some(logger) = &config.logger {
            if let Some(jailer_working_dir) = &self.jailer_working_dir {
                // using jailer
                // jailer_working_dir = <chroot_base>/<exec_file_name>/<id>/root

                // compute exported path
                let log_path = PathBuf::from(&logger.log_path);
                let log_path = if log_path.is_absolute() {
                    log_path.strip_prefix("/").map_err(|_| {
                        RtckError::Hypervisor("fail to strip absolute prefix".to_string())
                    })?
                } else {
                    log_path.as_path()
                };
                let log_path_external = jailer_working_dir.join(log_path);
                std::fs::File::create(&log_path_external).map_err(|_| {
                    RtckError::Hypervisor("fail to create logging file".to_string())
                })?;

                // using jailer, must change the owner of logger file to jailer uid:gid.
                use nix::unistd::{Gid, Uid};
                let (uid, gid) = self.uid_gid.ok_or(RtckError::Hypervisor(
                    "no uid and gid found in jailer".to_string(),
                ))?;
                let metadata = std::fs::metadata(&log_path_external).map_err(|_| {
                    RtckError::Hypervisor(format!(
                        "fail to get metadata of source path {:?}",
                        &log_path_external
                    ))
                })?;
                use std::os::unix::fs::MetadataExt;
                let original_uid = metadata.uid();
                let original_gid = metadata.gid();
                nix::unistd::chown(
                    &log_path_external,
                    Some(Uid::from_raw(uid)),
                    Some(Gid::from_raw(gid)),
                )
                .map_err(|_| {
                    RtckError::Hypervisor("fail to change the owner of jailer".to_string())
                })?;

                // rolling back if fail
                self.rollbacks.insert_1(Rollback::Chown {
                    path: log_path_external.clone(),
                    original_uid,
                    original_gid,
                });
            }

            // Logger's exported path is only useful when creating it and changing owner of it.
            // Now we could just use jailed path, so no need for changing `logger`.
            let put_logger = PutLogger::new(logger.clone());
            let res = self.agent.event(put_logger).map_err(|e| {
                // log::error!("PutLogger event failed");
                self.log(Error, "PutLogger event failed");
                e
            })?;
            if res.is_err() {
                // log::error!("PutLogger failed");
                self.log(Error, "PutLogger failed");
                return Err(RtckError::Hypervisor(
                    "fail to configure logger, rollback".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Metrics configuration. Nothing to rollback in this step.
    fn metrics_configure(&mut self, config: &MicroVMConfig) -> RtckResult<()> {
        if let Some(metrics) = &config.metrics {
            if let Some(jailer_working_dir) = &self.jailer_working_dir {
                // using jailer
                // jailer_working_dir = <chroot_base>/<exec_file_name>/<id>/root

                let metrics_path = PathBuf::from(&metrics.metrics_path);
                let metrics_path = if metrics_path.is_absolute() {
                    metrics_path.strip_prefix("/").map_err(|_| {
                        RtckError::Hypervisor("fail to strip absolute prefix".to_string())
                    })?
                } else {
                    metrics_path.as_path()
                };
                let metrics_path_external = jailer_working_dir.join(metrics_path);
                std::fs::File::create(&metrics_path_external).map_err(|_| {
                    RtckError::Hypervisor("fail to create logging file".to_string())
                })?;

                // using jailer, must change the owner of metrics file to jailer uid:gid.
                use nix::unistd::{Gid, Uid};
                let (uid, gid) = self.uid_gid.ok_or(RtckError::Hypervisor(
                    "no uid and gid found in jailer".to_string(),
                ))?;
                let metadata = std::fs::metadata(&metrics_path_external).map_err(|_| {
                    RtckError::Hypervisor(format!(
                        "fail to get metadata of source path {:?}",
                        &metrics_path_external
                    ))
                })?;
                use std::os::unix::fs::MetadataExt;
                let original_uid = metadata.uid();
                let original_gid = metadata.gid();
                nix::unistd::chown(
                    &metrics_path_external,
                    Some(Uid::from_raw(uid)),
                    Some(Gid::from_raw(gid)),
                )
                .map_err(|_| {
                    RtckError::Hypervisor("fail to change the owner of jailer".to_string())
                })?;

                // rolling back if fail
                self.rollbacks.insert_1(Rollback::Chown {
                    path: metrics_path_external.clone(),
                    original_uid,
                    original_gid,
                });
            }

            // Logger's exported path is only useful when creating it and changing owner of it.
            // Now we could just use jailed path, so no need for changing `logger`.
            let put_metrics = PutMetrics::new(metrics.clone());
            let res = self.agent.event(put_metrics).map_err(|e| {
                // log::error!("PutMetrics event failed");
                self.log(Error, "PutMetrics event failed");
                e
            })?;
            if res.is_err() {
                // log::error!("PutMetrics failed");
                self.log(Error, "PutMetrics failed");
                return Err(RtckError::Hypervisor(
                    "fail to configure metrics, rollback".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Guest boot source configuration.
    /// Roll back mounting kernel image directory if err.
    fn boot_source_configure(&mut self, config: &MicroVMConfig) -> RtckResult<()> {
        let boot_source = if let Some(jailer_working_dir) = &self.jailer_working_dir {
            // using jailer
            // jailer_working_dir = <chroot_base>/<exec_file_name>/<id>/root
            if let Some(boot_source) = &config.boot_source {
                // mount the kernel directory
                let target_dir = jailer_working_dir.join("kernel");
                std::fs::create_dir_all(&target_dir)
                    .map_err(|_| RtckError::Hypervisor("fail to create kernel dir".to_string()))?;
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
                            "fail to mount kernel image dir into jailer, errno = {e}",
                        )))
                    }
                }

                let mut boot_source = config.boot_source.clone().unwrap();
                let mut jailed_kernel_image_path = PathBuf::from("/kernel");
                jailed_kernel_image_path.push(kernel_file);
                boot_source.kernel_image_path =
                    jailed_kernel_image_path.to_string_lossy().to_string();

                // mount the initrd directory (if using initrd)
                if let Some(initrd_path) = &boot_source.initrd_path {
                    let target_dir = jailer_working_dir.join("initrd");
                    std::fs::create_dir_all(&target_dir).map_err(|_| {
                        RtckError::Hypervisor("fail to create initrd dir".to_string())
                    })?;
                    let source = PathBuf::from(initrd_path)
                        .canonicalize()
                        .map_err(|_| RtckError::Config("invalid path".to_string()))?;
                    let source_dir = source
                        .parent()
                        .ok_or(RtckError::Config("invalid path".to_string()))?;
                    let initrd_file = source.file_name().ok_or(RtckError::Config(
                        "invalid kernel image file path".to_string(),
                    ))?;

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
                        }
                        Err(e) => {
                            return Err(RtckError::Hypervisor(format!(
                                "fail to mount initrd dir into jailer, errno = {e}"
                            )))
                        }
                    }

                    let mut jailed_initrd_path = PathBuf::from("initrd");
                    jailed_initrd_path.push(initrd_file);
                    boot_source.initrd_path =
                        Some(jailed_initrd_path.to_string_lossy().to_string());
                }

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
            let res = self.agent.event(put_guest_boot_source).map_err(|e| {
                // log::error!("PutGuestBootSource event failed");
                self.log(Error, "PutGuestBootSource event failed");
                e
            })?;
            if res.is_err() {
                // log::error!("PutGuestBootSource failed");
                self.log(Error, "PutGuestBootSource failed");
                return Err(RtckError::Hypervisor(
                    "fail to configure boot source, rollback".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Guest drives configuration.
    /// Roll back mounting drives directory and changing owner if err.
    fn drives_configure(&mut self, config: &MicroVMConfig) -> RtckResult<()> {
        if let Some(drives) = &config.drives {
            for drive in drives {
                let drive = if let Some(jailer_working_dir) = &self.jailer_working_dir {
                    // using jailer
                    // jailer_working_dir = <chroot_base>/<exec_file_name>/<id>/root
                    // mount the drives directory
                    // TODO: mount the drive socket?
                    let target_dir = jailer_working_dir.join(format!("drives{}", drive.drive_id));
                    std::fs::create_dir(&target_dir).map_err(|_| {
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
                    nix::unistd::chown(&source, Some(Uid::from_raw(uid)), Some(Gid::from_raw(gid)))
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
                let res = self.agent.event(put_guest_drive_by_id).map_err(|e| {
                    // log::error!("PutGuestDriveByIDResponse event failed");
                    self.log(Error, "PutGuestDriveByIDResponse event failed");
                    e
                })?;
                if res.is_err() {
                    // log::error!("PutGuestDriveById failed");
                    self.log(Error, "PutGuestDriveById failed");
                    return Err(RtckError::Hypervisor(format!(
                        "fail to configure drive {}, rollback",
                        drive.drive_id
                    )));
                }
            }
        }
        Ok(())
    }

    /// Guest network interfaces.
    fn network_configure(&mut self, config: &MicroVMConfig) -> RtckResult<()> {
        if let Some(ifaces) = &config.network_interfaces {
            for iface in ifaces {
                let put_guest_network_interface_by_id =
                    PutGuestNetworkInterfaceByID::new(iface.clone());
                let res = self
                    .agent
                    .event(put_guest_network_interface_by_id)
                    .map_err(|e| {
                        // log::error!("PutGuestNetworkInterfaceByID event failed");
                        self.log(Error, "PutGuestNetworkInterfaceByID event failed");
                        e
                    })?;
                if res.is_err() {
                    // log::error!("PutGuestNetworkInterfaceById failed");
                    self.log(Error, "PutGuestNetworkInterfaceById failed");
                    return Err(RtckError::Hypervisor(format!(
                        "fail to configure network {}, rollback",
                        iface.iface_id
                    )));
                }
            }
        }
        Ok(())
    }

    /// Vsocks configuration.
    fn vsock_configure(&mut self, config: &MicroVMConfig) -> RtckResult<()> {
        if let Some(vsocks) = &config.vsock_devices {
            for vsock in vsocks {
                let put_guest_vsock = PutGuestVsock::new(vsock.clone());
                let res = self.agent.event(put_guest_vsock).map_err(|e| {
                    // log::error!("PutGuestVsock event failed");
                    self.log(Error, "PutGuestVsock event failed");
                    e
                })?;
                if res.is_err() {
                    // log::error!("PutGuestVsock failed");
                    self.log(Error, "PutGuestVsock failed");
                    return Err(RtckError::Hypervisor(format!(
                        "fail to configure vsock {:?}, rollback",
                        vsock.vsock_id
                    )));
                }
            }
        }
        Ok(())
    }

    /// CPU configuration.
    fn cpu_configure(&mut self, config: &MicroVMConfig) -> RtckResult<()> {
        if let Some(cpu_config) = &config.cpu_config {
            let put_cpu_configuration = PutCpuConfiguration::new(cpu_config.clone());
            let res = self.agent.event(put_cpu_configuration).map_err(|e| {
                // log::error!("PutCpuConfiguration event failed");
                self.log(Error, "PutCpuConfiguration event failed");
                e
            })?;
            if res.is_err() {
                // log::error!("PutCpuConfiguration failed");
                self.log(Error, "PutCpuConfiguration failed");
                return Err(RtckError::Hypervisor(
                    "fail to configure CPU, rollback".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Machine configuration.
    fn machine_configure(&mut self, config: &MicroVMConfig) -> RtckResult<()> {
        if let Some(machine_config) = &config.machine_config {
            let put_machine_configuration = PutMachineConfiguration::new(machine_config.clone());
            let res = self.agent.event(put_machine_configuration).map_err(|e| {
                // log::error!("PutMachineConfiguration event failed");
                self.log(Error, "PutMachineConfiguration event failed");
                e
            })?;
            if res.is_err() {
                // log::error!("PutMachineConfiguration failed");
                self.log(Error, "PutMachineConfiguration failed");
                return Err(RtckError::Hypervisor(
                    "fail to configure machine, rollback".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Balloon configure.
    fn balloon_configure(&mut self, config: &MicroVMConfig) -> RtckResult<()> {
        if let Some(balloon) = &config.balloon {
            let put_balloon = PutBalloon::new(balloon.clone());
            let res = self.agent.event(put_balloon).map_err(|e| {
                // log::error!("PutBalloon event failed");
                self.log(Error, "PutBalloon event failed");
                e
            })?;
            if res.is_err() {
                // log::error!("PutBalloon failed");
                self.log(Error, "PutBalloon failed");
                return Err(RtckError::Hypervisor(
                    "fail to configure balloon, rollback".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Entropy device configure.
    fn entropy_configure(&mut self, config: &MicroVMConfig) -> RtckResult<()> {
        if let Some(entropy_device) = &config.entropy_device {
            let put_entropy = PutEntropy::new(entropy_device.clone());
            let res = self.agent.event(put_entropy).map_err(|e| {
                // log::error!("PutEntropy event failed");
                self.log(Error, "PutEntropy event failed");
                e
            })?;
            if res.is_err() {
                // log::error!("PutEntropy failed");
                self.log(Error, "PutEntropy failed");
                return Err(RtckError::Hypervisor(
                    "fail to configure entropy device, rollback".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Initial mmds content configure.
    fn init_metadata_configure(&mut self, config: &MicroVMConfig) -> RtckResult<()> {
        if let Some(content) = &config.init_metadata {
            let put_mmds = PutMmds::new(content.clone());
            let res = self.agent.event(put_mmds).map_err(|e| {
                // log::error!("PutMmds event failed");
                self.log(Error, "PutMmds event failed");
                e
            })?;
            if res.is_err() {
                // log::error!("PutMmds failed");
                self.log(Error, "PutMmds failed");
                return Err(RtckError::Hypervisor(
                    "fail to configure initial metadata, rollback".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Automatically configure the machine.
    /// User must guarantee that `config` passed to the machine contains
    /// valid firecracker configuration (`frck_config`).
    fn configure(&mut self, config: &MicroVMConfig) -> RtckResult<()> {
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

        self.logger_configure(config)?;

        self.metrics_configure(config)?;

        self.boot_source_configure(config)?;

        self.drives_configure(config)?;

        self.network_configure(config)?;

        self.vsock_configure(config)?;

        self.cpu_configure(config)?;

        self.machine_configure(config)?;

        self.balloon_configure(config)?;

        self.entropy_configure(config)?;

        self.init_metadata_configure(config)?;

        Ok(())
    }

    /// Run a microVM instance by passing a microVM configuraion
    pub fn start(&mut self, config: &MicroVMConfig) -> RtckResult<()> {
        // change microVM status in hypervisor
        self.status = MicroVMStatus::Start;

        self.configure(config).map_err(|_| {
            // log::error!("start fail, {} {}", file!(), line!());
            self.log(Error, "start fail, cannot configure");
            // change microVM status in hypervisor
            self.status = MicroVMStatus::Failure;
            RtckError::Hypervisor("fail to start".to_string())
        })?;

        let start_machine = CreateSyncAction::new(InstanceActionInfo {
            action_type: ActionType::InstanceStart,
        });

        let res = self.agent.event(start_machine).map_err(|_| {
            {
                // log::error!("start fail, {} {}", file!(), line!());
                self.log(Error, "start fail, cannot start machine");
                // change microVM status in hypervisor
                self.status = MicroVMStatus::Failure;
                RtckError::Hypervisor("fail to start".to_string())
            }
        })?;

        if res.is_err() {
            // log::error!("start fail, {} {}", file!(), line!());
            self.log(Error, "start fail, cannot start machine");
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
    pub fn pause(&mut self) -> RtckResult<()> {
        if self.status != MicroVMStatus::Running {
            // log::warn!("can not pause a microVM that's not running");
            self.log(Warn, "can not pause a microVM that's not running");
            return Ok(());
        }

        let pause_machine = PatchVm::new(vm::VM_STATE_PAUSED);

        let res = self.agent.event(pause_machine).map_err(|_| {
            // log::error!("pause fail");
            self.log(Error, "pause fail");
            // no need for changing status here since pausing failure isn't a fatal error
            RtckError::Hypervisor("fail to pause".to_string())
        })?;

        if res.is_err() {
            // log::error!("pause fail");
            self.log(Error, "pause fail");
            Err(RtckError::Hypervisor("fail to pause".to_string()))
        } else {
            self.status = MicroVMStatus::Paused;
            Ok(())
        }
    }

    /// Resume the machine by notifying the hypervisor
    pub fn resume(&mut self) -> RtckResult<()> {
        if self.status != MicroVMStatus::Paused {
            // log::warn!("can not resume a microVM that's not paused");
            self.log(Warn, "can not resume a microVM that's not paused");
            return Ok(());
        }

        let resume_machine = PatchVm::new(vm::VM_STATE_RESUMED);

        let res = self.agent.event(resume_machine).map_err(|_| {
            // log::error!("resume fail");
            self.log(Error, "resume fail");
            RtckError::Machine("fail to resume".to_string())
        })?;

        if res.is_err() {
            // log::error!("resume fail");
            self.log(Error, "resume fail");
            Err(RtckError::Machine("fail to resume".to_string()))
        } else {
            self.status = MicroVMStatus::Running;
            Ok(())
        }
    }

    /// Stop the machine by notifying the hypervisor.
    /// Hypervisor should still be valid.
    pub fn stop(&mut self) -> RtckResult<()> {
        let stop_machine = CreateSyncAction::new(InstanceActionInfo {
            action_type: ActionType::SendCtrlAtlDel,
        });

        let res = self.agent.event(stop_machine).map_err(|_| {
            // log::error!("stop fail");
            self.log(Error, "stop fail");
            RtckError::Machine("fail to stop".to_string())
        })?;

        if res.is_err() {
            // log::error!("stop fail");
            self.log(Error, "stop fail");
            Err(RtckError::Hypervisor("fail to stop".to_string()))
        } else {
            self.status = MicroVMStatus::Stop;
            Ok(())
        }
    }

    pub fn wait(&mut self) -> RtckResult<ExitStatus> {
        self.child
            .wait()
            .map_err(|_| RtckError::Hypervisor("waiting hypervisor to exit".to_string()))
    }

    /// Wait for microVM to exit microVM voluntarily
    pub fn unused(&mut self) -> RtckResult<()> {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(self.poll_status_secs));
            let describe_metrics = DescribeInstance::new();
            let res = self.agent.event(describe_metrics);
            match res {
                Ok(res) => {
                    if res.is_err() {
                        continue;
                    } else {
                        // Safe: res is bound to be succ
                        let info = res.succ();
                        let status = info.state;
                        match status {
                            InstanceState::NotStarted => break,
                            InstanceState::Paused => continue,
                            InstanceState::Running => continue,
                        }
                    }
                }
                // If there's error trying to get status of instance,
                // then something has happened to firecracker, maybe
                // the stop of execution of hypervisor.
                // Exit anyway.
                Err(_) => break,
            }
        }

        Ok(())
    }

    /// Create a snapshot
    pub fn snapshot<P: AsRef<str>, Q: AsRef<str>>(
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

        let res = self.agent.event(create_snapshot)?;
        if res.is_err() {
            // log::error!("Machine::snapshot fail");
            self.log(Error, "snapshot fail");
            return Err(RtckError::Machine("fail to create snapshot".to_string()));
        }
        Ok(())
    }

    /// Delete the machine by notifying firecracker
    pub fn delete(self) -> RtckResult<()> {
        drop(self);

        // delete network TUN/TAP interface

        // delete cgroups (if exists)

        Ok(())
    }

    /// Check microVM status and sync with remote.
    pub fn sync_status(&mut self) -> MicroVMStatus {
        let describe_metrics = DescribeInstance::new();
        let res = self.agent.event(describe_metrics);
        match res {
            Ok(res) => {
                if res.is_err() {
                    self.status = MicroVMStatus::Failure;
                    MicroVMStatus::Failure
                } else {
                    // Safe: res is bound to be succ
                    let info = res.succ();
                    let status = info.state;
                    match status {
                        InstanceState::NotStarted => MicroVMStatus::Stop,
                        InstanceState::Paused => MicroVMStatus::Paused,
                        InstanceState::Running => MicroVMStatus::Running,
                    }
                }
            }
            Err(_) => MicroVMStatus::Failure,
        }
    }

    pub async fn check_status(&self) -> MicroVMStatus {
        self.status
    }
}

/// API for actions that is legal after MicroVM started.
impl Hypervisor {
    pub fn patch_balloon_stats_interval(
        &mut self,
        stats_polling_interval_s: i64,
    ) -> RtckResult<()> {
        let patch_balloon_stats_interval = PatchBalloonStatsInterval::new(BalloonStatsUpdate {
            stats_polling_interval_s,
        });
        let res = self
            .agent
            .event(patch_balloon_stats_interval)
            .map_err(|e| {
                // log::error!("PatchBalloonStatsInterval event failed");
                self.log(Error, "PatchBalloonStatsInterval event failed");
                e
            })?;
        if res.is_err() {
            // log::error!("PatchBalloonStatsInterval failed");
            self.log(Error, "PatchBalloonStatsInterval failed");
            return Err(RtckError::Hypervisor("fail to patch balloon".to_string()));
        }
        Ok(())
    }

    pub fn patch_balloon(&mut self, amount_mib: i64) -> RtckResult<()> {
        let patch_balloon = PatchBalloon::new(BalloonUpdate { amount_mib });
        let res = self.agent.event(patch_balloon).map_err(|e| {
            // log::error!("PatchBalloon event failed");
            self.log(Error, "PatchBalloon event failed");
            e
        })?;
        if res.is_err() {
            // log::error!("PatchBalloon failed");
            self.log(Error, "PatchBalloon failed");
            return Err(RtckError::Hypervisor("fail to patch balloon".to_string()));
        }
        Ok(())
    }

    pub fn patch_guest_drive_by_id(
        &mut self,
        drive_id: String,
        path_on_host: Option<String>,
        rate_limiter: Option<RateLimiter>,
    ) -> RtckResult<()> {
        let patch_guest_drive_by_id = PatchGuestDriveByID::new(PartialDrive {
            drive_id,
            path_on_host,
            rate_limiter,
        });
        let res = self.agent.event(patch_guest_drive_by_id).map_err(|e| {
            // log::error!("PatchGuestDriveByID event failed");
            self.log(Error, "PatchGuestDriveByID event failed");
            e
        })?;
        if res.is_err() {
            // log::error!("PatchGuestDriveByID failed");
            self.log(Error, "PatchGuestDriveByID failed");
            return Err(RtckError::Hypervisor(
                "fail to patch guest drive".to_string(),
            ));
        }
        Ok(())
    }

    pub fn patch_guest_network_interface_by_id(
        &mut self,
        iface_id: String,
        rx_rate_limiter: Option<RateLimiter>,
        tx_rate_limiter: Option<RateLimiter>,
    ) -> RtckResult<()> {
        let patch_guest_network_interface_by_id =
            PatchGuestNetworkInterfaceByID::new(PartialNetworkInterface {
                iface_id,
                rx_rate_limiter,
                tx_rate_limiter,
            });
        let res = self
            .agent
            .event(patch_guest_network_interface_by_id)
            .map_err(|e| {
                // log::error!("PatchGuestNetworkInterfaceByID event failed");
                self.log(Error, "PatchGuestNetworkInterfaceByID event failed");
                e
            })?;
        if res.is_err() {
            // log::error!("PatchGuestNetworkInterfaceByID failed");
            self.log(Error, "PatchGuestNetworkInterfaceByID failed");
            return Err(RtckError::Hypervisor(
                "fail to path guest network interface".to_string(),
            ));
        }
        Ok(())
    }

    pub fn patch_machine_configuration(
        &mut self,
        vcpu_count: isize,
        mem_size_mib: isize,
        cpu_template: Option<CPUTemplate>,
        ht_enabled: Option<bool>,
        track_dirty_pages: Option<bool>,
    ) -> RtckResult<()> {
        let patch_machine_configuration = PatchMachineConfiguration::new(MachineConfiguration {
            cpu_template,
            ht_enabled,
            mem_size_mib,
            track_dirty_pages,
            vcpu_count,
        });
        let res = self.agent.event(patch_machine_configuration).map_err(|e| {
            // log::error!("PatchMachineConfiguration event failed");
            self.log(Error, "PatchMachineConfiguration event failed");
            e
        })?;
        if res.is_err() {
            // log::error!("PatchMachineConfiguration failed");
            self.log(Error, "PatchMachineConfiguration failed");
            return Err(RtckError::Hypervisor(
                "fail to patch machine configuration".to_string(),
            ));
        }
        Ok(())
    }

    pub fn patch_mmds(&mut self, content: String) -> RtckResult<()> {
        let patch_mmds = PatchMmds::new(content);
        let res = self.agent.event(patch_mmds).map_err(|e| {
            // log::error!("PatchMmds event failed");
            self.log(Error, "PatchMmds event failed");
            e
        })?;
        if res.is_err() {
            // log::error!("PatchMmds failed");
            self.log(Error, "PatchMmds failed");
            return Err(RtckError::Hypervisor(
                "fail to patch mmds content".to_string(),
            ));
        }
        Ok(())
    }

    pub fn patch_vm(&mut self, state: VmState) -> RtckResult<()> {
        let patch_vm = PatchVm::new(Vm { state });
        let res = self.agent.event(patch_vm).map_err(|e| {
            // log::error!("PatchVm event failed");
            self.log(Error, "PatchVm event failed");
            e
        })?;
        if res.is_err() {
            // log::error!("PatchVm failed");
            self.log(Error, "PatchVm failed");
            return Err(RtckError::Hypervisor("fail to patch vm".to_string()));
        }
        Ok(())
    }
}
