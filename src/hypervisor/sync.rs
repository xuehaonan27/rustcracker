use super::MicroVMStatus;
use crate::agent::sync_agent::Agent;
use crate::config::{HypervisorConfig, MicroVMConfig};
use crate::firecracker::FirecrackerSync;
use crate::jailer::JailerSync;
use crate::models::*;
use crate::raii::{Rollback, RollbackStack};
use crate::reqres::*;
use crate::{RtckError, RtckResult};
use log::*;
use std::{path::PathBuf, process::ExitStatus};

pub struct Hypervisor {
    // instance id of the hypervisor process
    id: String,

    // pid of the hypervisor process
    pid: u32,

    // child of the hypervisor process
    child: std::process::Child,

    // socket path of this hypervisor
    socket_path: PathBuf,

    // retrying times
    socket_retry: usize,

    // lock path of this hypervisor
    lock_path: PathBuf,

    // log path of this hypervisor
    log_path: Option<PathBuf>,

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

    // rollback stack
    rollbacks: RollbackStack,

    // intervals in seconds for polling the status of microVM
    // when waiting for the user to give up the microVM
    poll_status_secs: u64,
}

impl Hypervisor {
    pub fn id(&self) -> &String {
        &self.id
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
    }

    pub fn socket_retry(&self) -> usize {
        self.socket_retry
    }

    pub fn lock_path(&self) -> &PathBuf {
        &self.lock_path
    }

    pub fn log_path(&self) -> Option<&PathBuf> {
        self.log_path.as_ref()
    }

    pub fn config_path(&self) -> Option<&String> {
        self.config_path.as_ref()
    }

    pub fn status(&self) -> MicroVMStatus {
        self.status
    }

    pub fn clear_jailer(&self) -> bool {
        self.clear_jailer
    }
}

impl Hypervisor {
    pub fn new(config: &HypervisorConfig) -> RtckResult<Self> {
        config.validate()?;
        if let Some(true) = config.using_jailer {
            Self::new_with_jailer(config)
        } else {
            Self::new_without_jailer(config)
        }
    }

    fn new_with_jailer(config: &HypervisorConfig) -> RtckResult<Self> {
        trace!("Creating instance with jailer");
        let mut rollbacks = RollbackStack::new();
        let mut jailer = JailerSync::from_config(&config)?;

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

        let stream = jailer.connect(config.socket_retry)?;

        let jailer_working_dir = jailer.get_jailer_workspace_dir().cloned();

        let uid = jailer.get_uid();

        let gid = jailer.get_gid();

        let uid_gid = Some((uid, gid));

        let firecracker = FirecrackerSync::from_jailer(jailer)?;

        // Shared code

        let lock = fslock::LockFile::open(&firecracker.lock_path).map_err(|e| {
            let msg = format!("Fail to create file lock: {e}");
            error!("{msg}");
            RtckError::Hypervisor(msg)
        })?;
        rollbacks.insert_1(Rollback::RemoveFsLock {
            path: firecracker.lock_path.clone(),
        });

        stream.set_nonblocking(true).map_err(|e| {
            let msg = format!("Unable to set stream to non-blocking: {e}");
            error!("{msg}");
            RtckError::Hypervisor(msg)
        })?;
        let agent = Agent::from_stream_lock(stream, lock);

        Ok(Self {
            id: firecracker.id,
            pid,
            child,
            socket_path: firecracker.socket,
            socket_retry: config.socket_retry,
            lock_path: firecracker.lock_path,
            log_path: firecracker.log_path,
            config_path: firecracker.config_path,
            agent,
            status: MicroVMStatus::None,
            clear_jailer,
            jailer_working_dir,
            uid_gid,
            rollbacks,
            poll_status_secs: config.poll_status_secs,
        })
    }

    fn new_without_jailer(config: &HypervisorConfig) -> RtckResult<Self> {
        trace!("Creating instance without jailer");
        let mut rollbacks = RollbackStack::new();
        let firecracker = FirecrackerSync::from_config(&config)?;

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

        let stream = firecracker.connect(config.socket_retry)?;

        // Shared code

        let lock = fslock::LockFile::open(&firecracker.lock_path).map_err(|e| {
            let msg = format!("Fail to create file lock: {e}");
            error!("{msg}");
            RtckError::Hypervisor(msg)
        })?;
        rollbacks.insert_1(Rollback::RemoveFsLock {
            path: firecracker.lock_path.clone(),
        });

        stream.set_nonblocking(true).map_err(|e| {
            let msg = format!("Unable to set stream to non-blocking: {e}");
            error!("{msg}");
            RtckError::Hypervisor(msg)
        })?;
        let agent = Agent::from_stream_lock(stream, lock);

        Ok(Self {
            id: firecracker.id,
            pid,
            child,
            socket_path: firecracker.socket,
            socket_retry: config.socket_retry,
            lock_path: firecracker.lock_path,
            log_path: firecracker.log_path,
            config_path: firecracker.config_path,
            agent,
            status: MicroVMStatus::None,
            clear_jailer: false,
            jailer_working_dir: None,
            uid_gid: None,
            rollbacks,
            poll_status_secs: config.poll_status_secs,
        })
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
                    log_path.strip_prefix("/").map_err(|e| {
                        let msg = format!("Fail to strip absolute prefix: {e}");
                        error!("{msg}");
                        RtckError::Hypervisor(msg)
                    })?
                } else {
                    log_path.as_path()
                };
                let log_path_external = jailer_working_dir.join(log_path);

                // create log file
                std::fs::File::create(&log_path_external).map_err(|e| {
                    let msg = format!("Fail to create log file: {e}");
                    error!("{msg}");
                    RtckError::Hypervisor(msg)
                })?;

                // using jailer, must change the owner of logger file to jailer uid:gid.
                use nix::unistd::{Gid, Uid};
                let (uid, gid) = self.uid_gid.ok_or_else(|| {
                    let msg = "Uid and Gid not found in jailer";
                    error!("msg");
                    RtckError::Hypervisor(msg.into())
                })?;
                let metadata = std::fs::metadata(&log_path_external).map_err(|e| {
                    let msg =
                        format!("Fail to get metadata of log file at {log_path_external:#?}: {e}");
                    error!("{msg}");
                    RtckError::Hypervisor(msg)
                })?;
                use std::os::unix::fs::MetadataExt;
                let original_uid = metadata.uid();
                let original_gid = metadata.gid();
                nix::unistd::chown(
                    &log_path_external,
                    Some(Uid::from_raw(uid)),
                    Some(Gid::from_raw(gid)),
                )
                .map_err(|e| {
                    let msg = format!("Fail to change the owner of log file: {e}");
                    error!("{msg}");
                    RtckError::Hypervisor(msg)
                })?;

                // rolling back if fail
                self.rollbacks.insert_1(Rollback::Chown {
                    path: log_path_external.clone(),
                    original_uid,
                    original_gid,
                });
            } else {
                let log_path = PathBuf::from(&logger.log_path);
                if !log_path.exists() {
                    // create log file
                    std::fs::File::create(&log_path).map_err(|e| {
                        let msg = format!("Fail to create log file: {e}");
                        error!("{msg}");
                        RtckError::Hypervisor(msg)
                    })?;
                }
            }

            // Logger exported path is only useful when creating it and changing owner of it.
            // Now we could just use jailed path, so no need for changing `logger`.
            let put_logger = PutLogger::new(logger.clone());
            let res = self.agent.event(put_logger).map_err(|e| {
                let msg = format!("PutLogger event failed: {e}");
                error!("{msg}");
                RtckError::Agent(e)
            })?;
            if res.is_err() {
                let msg = "PutLogger event returned error response";
                error!("{msg}");
                return Err(RtckError::Hypervisor(msg.into()));
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
                    metrics_path.strip_prefix("/").map_err(|e| {
                        let msg = format!("Fail to strip absolute prefix: {e}");
                        error!("{msg}");
                        RtckError::Hypervisor(msg)
                    })?
                } else {
                    metrics_path.as_path()
                };
                let metrics_path_external = jailer_working_dir.join(metrics_path);

                // create metrics file
                std::fs::File::create(&metrics_path_external).map_err(|e| {
                    let msg = format!("Fail to create metrics file: {e}");
                    error!("{msg}");
                    RtckError::Hypervisor(msg)
                })?;

                // using jailer, must change the owner of metrics file to jailer uid:gid.
                use nix::unistd::{Gid, Uid};
                let (uid, gid) = self.uid_gid.ok_or_else(|| {
                    let msg = "Uid and Gid not found in jailer";
                    error!("msg");
                    RtckError::Hypervisor(msg.into())
                })?;
                let metadata = std::fs::metadata(&metrics_path_external).map_err(|e| {
                    let msg = format!(
                        "Fail to get metadata of metrics file at {metrics_path_external:#?}: {e}"
                    );
                    error!("{msg}");
                    RtckError::Hypervisor(msg)
                })?;
                use std::os::unix::fs::MetadataExt;
                let original_uid = metadata.uid();
                let original_gid = metadata.gid();
                nix::unistd::chown(
                    &metrics_path_external,
                    Some(Uid::from_raw(uid)),
                    Some(Gid::from_raw(gid)),
                )
                .map_err(|e| {
                    let msg = format!("Fail to change the owner of metrics file: {e}");
                    error!("{msg}");
                    RtckError::Hypervisor(msg)
                })?;

                // rolling back if fail
                self.rollbacks.insert_1(Rollback::Chown {
                    path: metrics_path_external.clone(),
                    original_uid,
                    original_gid,
                });
            } else {
                let metrics_path = PathBuf::from(&metrics.metrics_path);
                if !metrics_path.exists() {
                    // create metrics file
                    std::fs::File::create(&metrics_path).map_err(|e| {
                        let msg = format!("Fail to create metrics file: {e}");
                        error!("{msg}");
                        RtckError::Hypervisor(msg)
                    })?;
                }
            }

            // Metrics exported path is only useful when creating it and changing owner of it.
            // Now we could just use jailed path, so no need for changing `logger`.
            let put_metrics = PutMetrics::new(metrics.clone());
            let res = self.agent.event(put_metrics).map_err(|e| {
                let msg = format!("PutMetrics event failed: {e}");
                error!("{msg}");
                RtckError::Agent(e)
            })?;
            if res.is_err() {
                let msg = "PutMetrics event returned error response";
                error!("{msg}");
                return Err(RtckError::Hypervisor(msg.into()));
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
                std::fs::create_dir_all(&target_dir).map_err(|e| {
                    let msg = format!("Fail to create kernel directory under jailer: {e}");
                    error!("{msg}");
                    RtckError::Hypervisor(msg)
                })?;

                let kerimg_path = &boot_source.kernel_image_path;
                let source = PathBuf::from(kerimg_path).canonicalize().map_err(|e| {
                    let msg = format!("Invalid kernel image path, got {kerimg_path}: {e}");
                    error!("{msg}");
                    RtckError::Config(msg)
                })?;
                let source_dir = source.parent().ok_or_else(|| {
                    let msg = format!("Invalid kernel image path, got {kerimg_path}");
                    error!("{msg}");
                    RtckError::Config(msg)
                })?;
                let kernel_file = source.file_name().ok_or_else(|| {
                    let msg = format!("Invalid kernel image path, got {kerimg_path}");
                    error!("{msg}");
                    RtckError::Config(msg)
                })?;

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
                        let msg = format!("Fail to mount kernel image dir into jailer: {e}");
                        error!("{msg}");
                        return Err(RtckError::Hypervisor(msg));
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
                    std::fs::create_dir_all(&target_dir).map_err(|e| {
                        let msg = format!("Fail to create initrd directory under jailer: {e}");
                        error!("{msg}");
                        RtckError::Hypervisor(msg)
                    })?;
                    let source = PathBuf::from(initrd_path).canonicalize().map_err(|e| {
                        let msg = format!("Invalid initrd path, got {initrd_path}: {e}");
                        error!("{msg}");
                        RtckError::Config(msg)
                    })?;
                    let source_dir = source.parent().ok_or_else(|| {
                        let msg = format!("Invalid initrd path, got {initrd_path}");
                        error!("{msg}");
                        RtckError::Config(msg)
                    })?;
                    let initrd_file = source.file_name().ok_or_else(|| {
                        let msg = format!("Invalid initrd path, got {initrd_path}");
                        error!("{msg}");
                        RtckError::Config(msg)
                    })?;

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
                            let msg = format!("Fail to mount initrd dir into jailer: {e}");
                            error!("{msg}");
                            return Err(RtckError::Hypervisor(msg));
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
            let put_guest_boot_source = PutGuestBootSource::new(boot_source.clone());
            let res = self.agent.event(put_guest_boot_source).map_err(|e| {
                let msg = format!("PutGuestBootSource event failed: {e}");
                error!("{msg}");
                RtckError::Agent(e)
            })?;
            if res.is_err() {
                let msg = "PutGuestBootSource event returned error response";
                error!("{msg}");
                return Err(RtckError::Hypervisor(msg.into()));
            }
        } else {
            let msg = "Must specify BootSource config in MicroVMConfig passed in";
            error!("{msg}");
            return Err(RtckError::Config(msg.into()));
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
                    std::fs::create_dir(&target_dir).map_err(|e| {
                        let msg = format!("Fail to create dir for drives {}: {e}", drive.drive_id);
                        error!("{msg}");
                        RtckError::Hypervisor(msg)
                    })?;
                    let drive_path = &drive.path_on_host;
                    let source = PathBuf::from(drive_path).canonicalize().map_err(|e| {
                        let msg = format!("Invalid drive path, got {drive_path}: {e}");
                        error!("{msg}");
                        RtckError::Config(msg)
                    })?;
                    let source_dir = source.parent().ok_or_else(|| {
                        let msg = format!("Invalid drive path, got {drive_path}");
                        error!("{msg}");
                        RtckError::Config(msg)
                    })?;
                    let drive_file = source.file_name().ok_or_else(|| {
                        let msg = format!("Invalid drive path, got {drive_path}");
                        error!("{msg}");
                        RtckError::Config(msg)
                    })?;

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
                        }
                        Err(e) => {
                            let msg = format!(
                                "Fail to mount drive {} dir into jailer: {e}",
                                drive.drive_id
                            );
                            error!("{msg}");
                            return Err(RtckError::Hypervisor(msg));
                        }
                    }

                    use nix::unistd::{Gid, Uid};
                    // change the owner of the drive
                    let (uid, gid) = self.uid_gid.ok_or_else(|| {
                        let msg = "Uid and Gid not found in jailer";
                        error!("{msg}");
                        RtckError::Hypervisor(msg.into())
                    })?;
                    let metadata = std::fs::metadata(&source).map_err(|e| {
                        let msg = format!("Fail to get metadata of drive at {source:#?}: {e}");
                        error!("{msg}");
                        RtckError::Hypervisor(msg)
                    })?;
                    use std::os::unix::fs::MetadataExt;
                    let original_uid = metadata.uid();
                    let original_gid = metadata.gid();
                    nix::unistd::chown(&source, Some(Uid::from_raw(uid)), Some(Gid::from_raw(gid)))
                        .map_err(|e| {
                            let msg = format!("Fail to change the owner of drive: {e}");
                            error!("{msg}");
                            RtckError::Hypervisor(msg)
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
                    let msg = format!("PutGuestDriveByID event failed: {e}");
                    error!("{msg}");
                    RtckError::Agent(e)
                })?;
                if res.is_err() {
                    let msg = format!(
                        "PutGuestDriveById event returned error response, drive {}",
                        drive.drive_id
                    );
                    error!("{msg}");
                    return Err(RtckError::Hypervisor(msg));
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
                        let msg = format!("PutGuestNetworkInterfaceByID event failed: {e}");
                        error!("{msg}");
                        RtckError::Agent(e)
                    })?;
                if res.is_err() {
                    let msg = format!(
                        "PutGuestNetworkInterfaceById event returned error response, iface {}",
                        iface.iface_id
                    );
                    error!("{msg}");
                    return Err(RtckError::Hypervisor(msg));
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
                    let msg = format!("PutGuestVsock event failed: {e}");
                    error!("{msg}");
                    RtckError::Agent(e)
                })?;
                if res.is_err() {
                    let msg = format!(
                        "PutGuestVsock event returned error response, vsock id {:#?}",
                        vsock.vsock_id
                    );
                    error!("{msg}");
                    return Err(RtckError::Hypervisor(msg));
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
                let msg = format!("PutCpuConfiguration event failed: {e}");
                error!("{msg}");
                RtckError::Agent(e)
            })?;
            if res.is_err() {
                let msg = "PutCpuConfiguration event returned error response";
                error!("{msg}");
                return Err(RtckError::Hypervisor(msg.into()));
            }
        }
        Ok(())
    }

    /// Machine configuration.
    fn machine_configure(&mut self, config: &MicroVMConfig) -> RtckResult<()> {
        if let Some(machine_config) = &config.machine_config {
            let put_machine_configuration = PutMachineConfiguration::new(machine_config.clone());
            let res = self.agent.event(put_machine_configuration).map_err(|e| {
                let msg = format!("PutMachineConfiguration event failed: {e}");
                error!("{msg}");
                RtckError::Agent(e)
            })?;
            if res.is_err() {
                let msg = "PutMachineConfiguration event returned error response";
                error!("{msg}");
                return Err(RtckError::Hypervisor(msg.into()));
            }
        }
        Ok(())
    }

    /// Balloon configure.
    fn balloon_configure(&mut self, config: &MicroVMConfig) -> RtckResult<()> {
        if let Some(balloon) = &config.balloon {
            let put_balloon = PutBalloon::new(balloon.clone());
            let res = self.agent.event(put_balloon).map_err(|e| {
                let msg = format!("PutBalloon event failed: {e}");
                error!("{msg}");
                RtckError::Agent(e)
            })?;
            if res.is_err() {
                let msg = "PutBalloon event returned error response";
                error!("{msg}");
                return Err(RtckError::Hypervisor(msg.into()));
            }
        }
        Ok(())
    }

    /// Entropy device configure.
    fn entropy_configure(&mut self, config: &MicroVMConfig) -> RtckResult<()> {
        if let Some(entropy_device) = &config.entropy_device {
            let put_entropy = PutEntropy::new(entropy_device.clone());
            let res = self.agent.event(put_entropy).map_err(|e| {
                let msg = format!("PutEntropy event failed: {e}");
                error!("{msg}");
                RtckError::Agent(e)
            })?;
            if res.is_err() {
                let msg = "PutEntropy event returned error response";
                error!("{msg}");
                return Err(RtckError::Hypervisor(msg.into()));
            }
        }
        Ok(())
    }

    /// Initial mmds content configure.
    fn init_metadata_configure(&mut self, config: &MicroVMConfig) -> RtckResult<()> {
        if let Some(content) = &config.init_metadata {
            let put_mmds = PutMmds::new(content.clone());
            let res = self.agent.event(put_mmds).map_err(|e| {
                let msg = format!("PutMmds event failed: {e}");
                error!("{msg}");
                RtckError::Agent(e)
            })?;
            if res.is_err() {
                let msg = "PutMmds event returned error response";
                error!("{msg}");
                return Err(RtckError::Hypervisor(msg.into()));
            }
        }
        Ok(())
    }

    /// Automatically configure the machine.
    /// User must guarantee that `config` passed to the machine contains
    /// valid firecracker configuration (`frck_config`).
    fn configure(&mut self, config: &MicroVMConfig) -> RtckResult<()> {
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

        self.configure(config).map_err(|e| {
            // change microVM status in hypervisor
            self.status = MicroVMStatus::Failure;
            let msg = format!("Fail to configure: {e}");
            error!("{msg}");
            RtckError::Hypervisor(msg)
        })?;

        let start_machine = CreateSyncAction::new(InstanceActionInfo {
            action_type: ActionType::InstanceStart,
        });

        let res = self.agent.event(start_machine).map_err(|e| {
            // change microVM status in hypervisor
            self.status = MicroVMStatus::Failure;
            let msg = format!("Start machine event failed: {e}");
            error!("{msg}");
            RtckError::Hypervisor(msg)
        })?;

        if res.is_err() {
            // change microVM status in hypervisor
            self.status = MicroVMStatus::Failure;
            let msg = "Start machine event returned error response";
            error!("{msg}");
            Err(RtckError::Hypervisor(msg.into()))
        } else {
            // change microVM status in hypervisor
            self.status = MicroVMStatus::Running;
            Ok(())
        }
    }

    /// Pause the machine by notifying the hypervisor
    pub fn pause(&mut self) -> RtckResult<()> {
        if self.status != MicroVMStatus::Running {
            warn!("Cannot pause a microVM which is not running");
            return Ok(());
        }

        let pause_machine = PatchVm::new(vm::VM_STATE_PAUSED);

        let res = self.agent.event(pause_machine).map_err(|e| {
            // no need for changing status here since pausing failure isn't a fatal error
            let msg = format!("Pause machine event failed: {e}");
            error!("{msg}");
            RtckError::Agent(e)
        })?;

        if res.is_err() {
            let msg = "Pause machine event returned error response";
            error!("{msg}");
            Err(RtckError::Hypervisor(msg.into()))
        } else {
            self.status = MicroVMStatus::Paused;
            Ok(())
        }
    }

    /// Resume the machine by notifying the hypervisor
    pub fn resume(&mut self) -> RtckResult<()> {
        if self.status != MicroVMStatus::Paused {
            warn!("Cannot resume a microVM which is not paused");
            return Ok(());
        }

        let resume_machine = PatchVm::new(vm::VM_STATE_RESUMED);

        let res = self.agent.event(resume_machine).map_err(|e| {
            // no need for changing status here since resuming failure isn't a fatal error
            let msg = format!("Resume machine event failed: {e}");
            error!("{msg}");
            RtckError::Agent(e)
        })?;

        if res.is_err() {
            let msg = "Resume machine event returned error response";
            error!("{msg}");
            Err(RtckError::Hypervisor(msg.into()))
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

        let res = self.agent.event(stop_machine).map_err(|e| {
            // change microVM status in hypervisor
            self.status = MicroVMStatus::Failure;
            let msg = format!("Fail to stop: {e}");
            error!("{msg}");
            RtckError::Hypervisor(msg)
        })?;

        if res.is_err() {
            // change microVM status in hypervisor
            self.status = MicroVMStatus::Failure;
            let msg = "Stop machine event returned error response";
            error!("{msg}");
            Err(RtckError::Hypervisor(msg.into()))
        } else {
            self.status = MicroVMStatus::Stop;
            Ok(())
        }
    }

    pub fn wait(&mut self) -> RtckResult<ExitStatus> {
        self.child.wait().map_err(|e| {
            let msg = format!("Fail to wait hypervisor to exit: {e}");
            error!("{msg}");
            RtckError::Hypervisor(msg)
        })
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

        let res = self.agent.event(create_snapshot).map_err(|e| {
            let msg = format!("CreateSnapshot event failed: {e}");
            error!("{msg}");
            RtckError::Agent(e)
        })?;
        if res.is_err() {
            let msg = "CreateSnapshot event returned error response";
            error!("{msg}");
            return Err(RtckError::Hypervisor(msg.into()));
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
                let msg = format!("PatchBalloonStatsInterval event failed: {e}");
                error!("{msg}");
                RtckError::Agent(e)
            })?;
        if res.is_err() {
            let msg = "PatchBalloonStatsInterval event returned error response";
            error!("{msg}");
            return Err(RtckError::Hypervisor(msg.into()));
        }
        Ok(())
    }

    pub fn patch_balloon(&mut self, amount_mib: i64) -> RtckResult<()> {
        let patch_balloon = PatchBalloon::new(BalloonUpdate { amount_mib });
        let res = self.agent.event(patch_balloon).map_err(|e| {
            let msg = format!("PatchBalloon event failed: {e}");
            error!("{msg}");
            RtckError::Agent(e)
        })?;
        if res.is_err() {
            let msg = "PatchBalloon event returned error response";
            error!("{msg}");
            return Err(RtckError::Hypervisor(msg.into()));
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
            let msg = format!("PatchGuestDriveByID event failed: {e}");
            error!("{msg}");
            RtckError::Agent(e)
        })?;
        if res.is_err() {
            let msg = "PatchGuestDriveByID event returned error response";
            error!("{msg}");
            return Err(RtckError::Hypervisor(msg.into()));
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
                let msg = format!("PatchGuestNetworkInterfaceByID event failed: {e}");
                error!("{msg}");
                RtckError::Agent(e)
            })?;
        if res.is_err() {
            let msg = "PatchGuestNetworkInterfaceByID event returned error response";
            error!("{msg}");
            return Err(RtckError::Hypervisor(msg.into()));
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
            let msg = format!("PatchMachineConfiguration event failed: {e}");
            error!("{msg}");
            RtckError::Agent(e)
        })?;
        if res.is_err() {
            let msg = "PatchMachineConfiguration event returned error response";
            error!("{msg}");
            return Err(RtckError::Hypervisor(msg.into()));
        }
        Ok(())
    }

    pub fn patch_mmds(&mut self, content: String) -> RtckResult<()> {
        let patch_mmds = PatchMmds::new(content);
        let res = self.agent.event(patch_mmds).map_err(|e| {
            let msg = format!("PatchMmds event failed: {e}");
            error!("{msg}");
            RtckError::Agent(e)
        })?;
        if res.is_err() {
            let msg = "PatchMmds event returned error response";
            error!("{msg}");
            return Err(RtckError::Hypervisor(msg.into()));
        }
        Ok(())
    }

    pub fn patch_vm(&mut self, state: VmState) -> RtckResult<()> {
        let patch_vm = PatchVm::new(Vm { state });
        let res = self.agent.event(patch_vm).map_err(|e| {
            let msg = format!("PatchVm event failed: {e}");
            error!("{msg}");
            RtckError::Agent(e)
        })?;
        if res.is_err() {
            let msg = "PatchVm event returned error response";
            error!("{msg}");
            return Err(RtckError::Hypervisor(msg.into()));
        }
        Ok(())
    }
}
