use std::{
    any::{Any, TypeId},
    os::fd::FromRawFd,
    path::PathBuf,
};

use crate::client::jailer::{DEFAULT_JAILER_PATH, ROOTFS_FOLDER_NAME};

use super::{
    jailer::StdioTypes,
    machine::{Machine, MachineError},
};

use log::{debug, error, warn};
use nix::{
    fcntl::{self, OFlag},
    sys::stat::Mode,
    unistd,
};
use serde::{Deserialize, Serialize};

pub trait HandlerName {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StartVMMHandlerName;
impl HandlerName for StartVMMHandlerName {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BootstrapLoggingHandlerName;
impl HandlerName for BootstrapLoggingHandlerName {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CreateLogFilesHandlerName;
impl HandlerName for CreateLogFilesHandlerName {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CreateMachineHandlerName;
impl HandlerName for CreateMachineHandlerName {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CreateBootSourceHandlerName;
impl HandlerName for CreateBootSourceHandlerName {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AttachDrivesHandlerName;
impl HandlerName for AttachDrivesHandlerName {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CreateNetworkInterfacesHandlerName;
impl HandlerName for CreateNetworkInterfacesHandlerName {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AddVsocksHandlerName;
impl HandlerName for AddVsocksHandlerName {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SetMetadataHandlerName;
impl HandlerName for SetMetadataHandlerName {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConfigMmdsHandlerName;
impl HandlerName for ConfigMmdsHandlerName {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LinkFilesToRootFSHandlerName;
impl HandlerName for LinkFilesToRootFSHandlerName {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SetupNetworkHandlerName;
impl HandlerName for SetupNetworkHandlerName {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SetupKernelArgsHandlerName;
impl HandlerName for SetupKernelArgsHandlerName {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CreateBalloonHandlerName;
impl HandlerName for CreateBalloonHandlerName {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ValidateCfgHandlerName;
impl HandlerName for ValidateCfgHandlerName {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ValidateJailerCfgHandlerName;
impl HandlerName for ValidateJailerCfgHandlerName {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ValidateNetworkCfgHandlerName;
impl HandlerName for ValidateNetworkCfgHandlerName {}

/// Handler are records that's put into Machine instances,
/// instructing preparations and cleanings.
#[derive(Debug, Clone)]
pub enum Handler {
    /// ConfigValidationHandler is used to validate that required fields are
    /// present. This validator is to be used when the jailer is turned off.
    ConfigValidationHandler {
        name: ValidateCfgHandlerName,
    },

    /// JailerConfigValidationHandler is used to validate that required fields are
    /// present.
    JailerConfigValidationHandler {
        name: ValidateJailerCfgHandlerName,
    },
    NetworkConfigValidationHandler {
        name: ValidateNetworkCfgHandlerName,
    },

    /// StartVMMHandler is a named handler that will handle starting of the VMM.
    /// This handler will also set the exit channel on completion.
    StartVMMHandler {
        name: StartVMMHandlerName,
    },

    /// CreateLogFilesHandler is a named handler that will create the fifo log files
    /// and redirect stdout and stderr to log fifo or log path specified by the machine configuration
    CreateLogFilesHandler {
        name: CreateLogFilesHandlerName,
    },

    /// BootstrapLoggingHandler is a named handler that will set up fifo logging of
    /// firecracker process.
    BootstrapLoggingHandler {
        name: BootstrapLoggingHandlerName,
    },

    /// CreateMachineHandler is a named handler that will "create" the machine and
    /// upload any necessary configuration to the firecracker process.
    CreateMachineHandler {
        name: CreateMachineHandlerName,
    },

    /// CreateBootSourceHandler is a named handler that will set up the booting
    /// process of the firecracker process.
    CreateBootSourceHandler {
        name: CreateBootSourceHandlerName,
    },

    /// AttachDrivesHandler is a named handler that will attach all drives for the
    /// firecracker process.
    AttachDrivesHandler {
        name: AttachDrivesHandlerName,
    },

    /// CreateNetworkInterfacesHandler is a named handler that registers network
    /// interfaces with the Firecracker VMM.
    CreateNetworkInterfacesHandler {
        name: CreateNetworkInterfacesHandlerName,
    },

    /// SetupNetworkHandler is a named handler that will setup the network namespace
    /// and network interface configuration prior to the Firecracker VMM starting.
    SetupNetworkHandler {
        name: SetupNetworkHandlerName,
    },

    /// SetupKernelArgsHandler is a named handler that will update any kernel boot
    /// args being provided to the VM based on the other configuration provided, if
    /// needed.
    SetupKernelArgsHandler {
        name: SetupKernelArgsHandlerName,
    },

    /// AddVsocksHandler is a named handler that adds vsocks to the firecracker
    /// process.
    AddVsocksHandler {
        name: AddVsocksHandlerName,
    },

    /// NewSetMetadataHandler is a named handler that puts the metadata into the
    /// firecracker process.
    NewSetMetadataHandler {
        name: SetMetadataHandlerName,
        data: String,
    },

    /// ConfigMmdsHandler is a named handler that puts the MMDS config into the
    /// firecracker process.
    ConfigMmdsHandler {
        name: ConfigMmdsHandlerName,
    },

    /// NewCreateBalloonHandler is a named handler that put a memory balloon into the
    /// firecracker process.
    NewCreateBalloonHandler {
        name: CreateBalloonHandlerName,
        amount_mib: i64,
        deflate_on_oom: bool,
        stats_polling_interval_s: i64,
    },

    /// LinkFilesHandler creates a new link files handler that will link files to
    /// the rootfs
    LinkFilesHandler {
        name: LinkFilesToRootFSHandlerName,
        kernel_image_file_name: PathBuf,
    },
}

pub struct HandlerList(Vec<Handler>);

pub struct Handlers {
    pub validation: HandlerList,
    pub fcinit: HandlerList,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum HandlersAdapter {
    /// NaiveChrootStrategy will simply hard link all files, drives and kernel
    /// image, to the root drive.
    NaiveChrootStrategy {
        rootfs: PathBuf,
        kernel_image_path: PathBuf,
    },
}

impl HandlersAdapter {
    pub fn adapt_handlers(&self, handlers: &mut Handlers) -> Result<(), MachineError> {
        match self {
            // AdaptHandlers will inject the LinkFilesHandler into the handler list.
            HandlersAdapter::NaiveChrootStrategy {
                rootfs,
                kernel_image_path,
            } => {
                if !handlers.fcinit.has(CreateLogFilesHandlerName.type_id()) {
                    return Err(MachineError::Initialize(
                        "required handler is missing from FcInit's list".to_string(),
                    ));
                }
                handlers.fcinit.append_after(
                    CreateLogFilesHandlerName.type_id(),
                    &Handler::LinkFilesHandler {
                        name: LinkFilesToRootFSHandlerName,
                        kernel_image_file_name: kernel_image_path
                            .as_path()
                            .file_name()
                            .ok_or(MachineError::FileError(format!(
                                "fail to get file base name from {}",
                                kernel_image_path.display()
                            )))?
                            .into(),
                    },
                );
                Ok(())
            }
        }
    }
}

impl Handler {
    pub fn name(&self) -> TypeId {
        match self {
            Handler::ConfigValidationHandler { name } => name.type_id(),
            Handler::JailerConfigValidationHandler { name } => name.type_id(),
            Handler::NetworkConfigValidationHandler { name } => name.type_id(),
            Handler::StartVMMHandler { name } => name.type_id(),
            Handler::CreateLogFilesHandler { name } => name.type_id(),
            Handler::BootstrapLoggingHandler { name } => name.type_id(),
            Handler::CreateMachineHandler { name } => name.type_id(),
            Handler::CreateBootSourceHandler { name } => name.type_id(),
            Handler::AttachDrivesHandler { name } => name.type_id(),
            Handler::CreateNetworkInterfacesHandler { name } => name.type_id(),
            Handler::SetupNetworkHandler { name } => name.type_id(),
            Handler::SetupKernelArgsHandler { name } => name.type_id(),
            Handler::AddVsocksHandler { name } => name.type_id(),
            Handler::NewSetMetadataHandler { name, .. } => name.type_id(),
            Handler::ConfigMmdsHandler { name } => name.type_id(),
            Handler::NewCreateBalloonHandler { name, .. } => name.type_id(),
            Handler::LinkFilesHandler { name, .. } => name.type_id(),
        }
    }
    pub async fn func(&self, m: &mut Machine) -> Result<(), MachineError> {
        match self {
            Handler::ConfigValidationHandler { .. } => m.cfg.validate(),
            Handler::JailerConfigValidationHandler { .. } => {
                if m.cfg.jailer_cfg.is_none() {
                    return Ok(());
                }

                let mut has_root = m.cfg.initrd_path.is_some();
                for drive in m.cfg.drives.as_ref().unwrap() {
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

                if m.cfg.jailer_cfg.as_ref().unwrap().chroot_strategy.is_none() {
                    error!("chroot_strategy cannot be none");
                    return Err(MachineError::Validation(
                        "chroot_startegy cannot be none".to_string(),
                    ));
                }

                if m.cfg.jailer_cfg.as_ref().unwrap().exec_file.is_none() {
                    error!("exec file must be specified when using jailer mode");
                    return Err(MachineError::Validation(
                        "exec file must be specified when using jailer mode".to_string(),
                    ));
                }

                if m.cfg.jailer_cfg.as_ref().unwrap().id.is_none()
                    || m.cfg
                        .jailer_cfg
                        .as_ref()
                        .unwrap()
                        .id
                        .as_ref()
                        .unwrap()
                        .len()
                        == 0
                {
                    error!("id must be specified when using jailer mode");
                    return Err(MachineError::Validation(
                        "id must be specified when using jailer mode".to_string(),
                    ));
                }

                if m.cfg.jailer_cfg.as_ref().unwrap().gid.is_none() {
                    error!("gid must be specified when using jailer mode");
                    return Err(MachineError::Validation(
                        "gid must be specified when using jailer mode".to_string(),
                    ));
                }

                if m.cfg.jailer_cfg.as_ref().unwrap().uid.is_none() {
                    error!("uid must be specified when using jailer mode");
                    return Err(MachineError::Validation(
                        "uid must be specified when using jailer mode".to_string(),
                    ));
                }

                if m.cfg.jailer_cfg.as_ref().unwrap().numa_node.is_none() {
                    error!("numa node must be specified when using jailer mode");
                    return Err(MachineError::Validation(
                        "numa node must be specified when using jailer mode".to_string(),
                    ));
                }
                Ok(())
            }
            Handler::NetworkConfigValidationHandler { .. } => m.cfg.validate_network(),
            Handler::StartVMMHandler { .. } => m.start_vmm().await,
            Handler::CreateLogFilesHandler { .. } => {
                create_fifo_or_file(m, &m.cfg.metrics_fifo, &m.cfg.metrics_path)?;
                create_fifo_or_file(&m, &m.cfg.log_fifo, &m.cfg.log_path)?;
                if m.cfg.fifo_log_writer.is_some() {
                    // 将microVM输出重定向到log fifo和log path
                    todo!()
                }
                todo!()
            }
            Handler::BootstrapLoggingHandler { .. } => {
                m.setup_logging().await?;
                m.setup_metrics().await?;

                debug!("setup logging: success");
                Ok(())
            }
            Handler::CreateMachineHandler { .. } => m.create_machine().await,
            Handler::CreateBootSourceHandler { .. } => {
                m.create_boot_source(
                    m.cfg.kernel_image_path.as_ref().unwrap(),
                    m.cfg.initrd_path.as_ref().unwrap(),
                    m.cfg.kernel_args.as_ref().unwrap(),
                )
                .await
            }
            Handler::AttachDrivesHandler { .. } => m.attach_drives().await,
            Handler::CreateNetworkInterfacesHandler { .. } => m.create_network_interfaces().await,
            Handler::SetupNetworkHandler { .. } => m.setup_network().await,
            Handler::SetupKernelArgsHandler { .. } => m.setup_kernel_args().await,
            Handler::AddVsocksHandler { .. } => m.add_vsocks().await,
            Handler::NewSetMetadataHandler { name: _, data } => m.set_metadata(data).await,
            Handler::ConfigMmdsHandler { .. } => {
                m.set_mmds_config(m.cfg.mmds_address.as_ref().unwrap())
                    .await
            }
            Handler::NewCreateBalloonHandler {
                name: _,
                amount_mib,
                deflate_on_oom,
                stats_polling_interval_s,
            } => {
                m.create_balloon(*amount_mib, *deflate_on_oom, *stats_polling_interval_s)
                    .await
            }
            Handler::LinkFilesHandler {
                name: _,
                kernel_image_file_name,
            } => {
                if m.cfg.jailer_cfg.is_none() {
                    return Err(MachineError::FileError(
                        "jailer config was not set for use".to_string(),
                    ));
                }

                let rootfs: PathBuf = [
                    m.cfg
                        .jailer_cfg
                        .as_mut()
                        .unwrap()
                        .chroot_base_dir
                        .to_owned()
                        .unwrap_or(DEFAULT_JAILER_PATH.into()),
                    m.cfg
                        .jailer_cfg
                        .as_mut()
                        .unwrap()
                        .exec_file
                        .as_ref()
                        .unwrap()
                        .as_path()
                        .file_name()
                        .ok_or(MachineError::FileError(format!(
                            "malformed firecracker exec file name"
                        )))?
                        .into(),
                    m.cfg
                        .jailer_cfg
                        .as_ref()
                        .unwrap()
                        .id
                        .as_ref()
                        .unwrap()
                        .into(),
                    ROOTFS_FOLDER_NAME.into(),
                ]
                .iter()
                .collect();

                // copy kernel image to root fs
                std::fs::hard_link(
                    &m.cfg.kernel_image_path.as_ref().unwrap(),
                    [&rootfs, &kernel_image_file_name.to_owned().into()]
                        .iter()
                        .collect::<PathBuf>(),
                )
                .map_err(|e| {
                    error!("fail to copy kernel image to root fs: {}", e.to_string());
                    MachineError::FileError(format!(
                        "fail to copy kernel image to root fs: {}",
                        e.to_string()
                    ))
                })?;

                let mut initrd_file_name: PathBuf = "".into();
                if m.cfg.initrd_path.is_some()
                    && m.cfg.initrd_path.to_owned().unwrap().as_os_str() != ""
                {
                    initrd_file_name = m
                        .cfg
                        .initrd_path
                        .to_owned()
                        .unwrap()
                        .as_path()
                        .file_name()
                        .ok_or(MachineError::FileError(format!("malformed initrd path")))?
                        .into();
                    std::fs::hard_link(
                        &m.cfg.initrd_path.as_mut().unwrap(),
                        [&rootfs, &initrd_file_name].iter().collect::<PathBuf>(),
                    )
                    .map_err(|e| {
                        error!("fail to copy initrd device to root fs: {}", e.to_string());
                        MachineError::FileError(format!(
                            "fail to copy initrd device to root fs: {}",
                            e.to_string()
                        ))
                    })?;
                }

                // copy all drives to the root fs
                for drive in m.cfg.drives.as_mut().unwrap() {
                    let host_path = &drive.get_path_on_host();
                    let drive_file_name: PathBuf = host_path
                        .as_path()
                        .file_name()
                        .ok_or(MachineError::FileError(
                            "malformed drive file name".to_string(),
                        ))?
                        .into();

                    std::fs::hard_link(
                        host_path,
                        [&rootfs, &drive_file_name].iter().collect::<PathBuf>(),
                    )
                    .map_err(|e| {
                        error!("fail to copy drives to root fs: {}", e.to_string());
                        MachineError::FileError(format!(
                            "fail to copy drives to root fs: {}",
                            e.to_string()
                        ))
                    })?;
                    // drive.path_on_host = drive_file_name;
                    drive.set_drive_path(drive_file_name);
                }

                // Modify Machine configuration
                m.cfg.kernel_image_path = kernel_image_file_name.to_owned().into();
                if m.cfg.initrd_path.is_some()
                    && m.cfg.initrd_path.as_mut().unwrap().as_os_str() != ""
                {
                    m.cfg.initrd_path = Some(initrd_file_name);
                }

                for fifo_path in [&mut m.cfg.log_fifo, &mut m.cfg.metrics_fifo] {
                    if fifo_path.is_none() || fifo_path.as_ref().unwrap().as_os_str() == "" {
                        continue;
                    }

                    let file_name: PathBuf = fifo_path
                        .as_mut()
                        .unwrap()
                        .as_path()
                        .file_name()
                        .ok_or(MachineError::FileError("malformed fifo path".to_string()))?
                        .into();
                    std::fs::hard_link(
                        fifo_path.as_mut().unwrap(),
                        [&rootfs, &file_name].iter().collect::<PathBuf>(),
                    )
                    .map_err(|e| {
                        error!("fail to copy fifo file to root fs: {}", e.to_string());
                        MachineError::FileError(format!(
                            "fail to copy fifo file to root fs: {}",
                            e.to_string()
                        ))
                    })?;

                    nix::unistd::chown(
                        &[&rootfs, &file_name].iter().collect::<PathBuf>(),
                        Some(nix::unistd::Uid::from_raw(
                            *m.cfg.jailer_cfg.as_ref().unwrap().uid.as_ref().unwrap() as u32,
                        )),
                        Some(nix::unistd::Gid::from_raw(
                            *m.cfg.jailer_cfg.as_ref().unwrap().gid.as_ref().unwrap() as u32,
                        )),
                    )
                    .map_err(|e| {
                        error!("fail to chown: {}", e.to_string());
                        MachineError::FileError(format!("fail to chown: {}", e.to_string()))
                    })?;

                    // update fifoPath as jailer works relative to the chroot dir
                    *fifo_path = Some(file_name);
                }

                Ok(())
            }
        }
    }
}

/*
cleanup functions

关闭logfifo通道, 移除logfifo文件
关闭metrics通道, 移除metrics文件

*/

fn create_fifo_or_file(
    machine: &Machine,
    fifo: &Option<PathBuf>,
    path: &Option<PathBuf>,
) -> Result<(), MachineError> {
    if let Some(fifo) = fifo {
        unistd::mkfifo(fifo, Mode::S_IRUSR | Mode::S_IWUSR).map_err(|e| {
            MachineError::FileError(format!(
                "cannot make fifo at {}: {}",
                fifo.display(),
                e.to_string()
            ))
        })?;

        todo!();
        /*
        m.cleanupFuncs = append(m.cleanupFuncs,
            func() error {
                if err := os.Remove(fifo); !os.IsNotExist(err) {
                    return err
                }
                return nil
            },
        )
         */

        Ok(())
    } else if let Some(path) = path {
        let raw_fd = fcntl::open(
            path,
            fcntl::OFlag::O_RDWR | fcntl::OFlag::O_CREAT | fcntl::OFlag::O_APPEND,
            Mode::S_IRUSR | Mode::S_IWUSR,
        )
        .map_err(|e| MachineError::FileError(format!("cannot make file: {}", e.to_string())))?;
        unistd::close(raw_fd).map_err(|e| {
            MachineError::FileError(format!(
                "fail to close file at {}: {}",
                path.display(),
                e.to_string()
            ))
        })?;
        Ok(())
    } else {
        Err(MachineError::FileError(
            "create_fifo_or_file: parameters wrong".into(),
        ))
    }
}

async fn capture_fifo_to_file(
    machine: &mut Machine,
    fifo_path: &PathBuf,
    w: StdioTypes,
) -> Result<(), MachineError> {
    // open the fifo pipe which will be used to write its contents to a file.
    let fifo_raw_fd = nix::fcntl::open(
        fifo_path,
        OFlag::O_RDONLY | OFlag::O_NONBLOCK,
        Mode::S_IRUSR | Mode::S_IWUSR, // 0o600
    )
    .map_err(|e| {
        error!(
            "Failed to open fifo path at {}, errno: {}",
            fifo_path.display(),
            e.to_string()
        );
        MachineError::FileError(format!(
            "Failed to open fifo path at {}, errno: {}",
            fifo_path.display(),
            e.to_string()
        ))
    })?;
    let fifo_pipe = unsafe { std::fs::File::from_raw_fd(fifo_raw_fd) };

    todo!()
}

impl HandlerList {
    pub fn default_fcinit_handler_list() -> Self {
        HandlerList(vec![
            Handler::SetupNetworkHandler {
                name: SetupNetworkHandlerName,
            },
            Handler::SetupKernelArgsHandler {
                name: SetupKernelArgsHandlerName,
            },
            Handler::StartVMMHandler {
                name: StartVMMHandlerName,
            },
            Handler::CreateLogFilesHandler {
                name: CreateLogFilesHandlerName,
            },
            Handler::BootstrapLoggingHandler {
                name: BootstrapLoggingHandlerName,
            },
            Handler::CreateMachineHandler {
                name: CreateMachineHandlerName,
            },
            Handler::CreateBootSourceHandler {
                name: CreateBootSourceHandlerName,
            },
            Handler::AttachDrivesHandler {
                name: AttachDrivesHandlerName,
            },
            Handler::CreateNetworkInterfacesHandler {
                name: CreateNetworkInterfacesHandlerName,
            },
            Handler::AddVsocksHandler {
                name: AddVsocksHandlerName,
            },
        ])
    }

    pub fn default_validation_handler_list() -> Self {
        HandlerList(vec![Handler::NetworkConfigValidationHandler {
            name: ValidateNetworkCfgHandlerName,
        }])
    }

    /// prepend will prepend a new set of handlers to the handler list
    pub fn prepend(&mut self, mut handlers: Vec<Handler>) {
        self.0.reverse();
        handlers.reverse();
        self.0.append(&mut handlers);
        self.0.reverse();
    }

    /// append will append new handlers to the handler list
    pub fn append(&mut self, mut handlers: Vec<Handler>) {
        self.0.append(&mut handlers);
    }

    /// append_after will append a given handler after the specified handler
    pub fn append_after(&mut self, name: TypeId, handler: &Handler) {
        let mut new_list: Vec<Handler> = Vec::new();
        for h in &self.0 {
            if h.name() == name {
                new_list.push(h.to_owned());
                new_list.push(handler.to_owned());
            } else {
                new_list.push(h.to_owned())
            }
        }
        self.0 = new_list;
    }

    /// len return the length of the given handler list
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// has will iterate through the handler list and check to see if the the named
    /// handler exists
    pub fn has(&self, name: TypeId) -> bool {
        for h in &self.0 {
            if h.name() == name {
                return true;
            }
        }

        return false;
    }

    /// replace will replace all elements of the given name with the new handler
    pub fn replace(&mut self, handler: &Handler) {
        let mut new_list: Vec<Handler> = Vec::new();
        for h in &self.0 {
            if h.name() == handler.name() {
                new_list.push(handler.to_owned());
            } else {
                new_list.push(h.to_owned());
            }
        }

        self.0 = new_list;
    }

    /// replacend will either append, if there isn't an element within the handler
    /// list, otherwise it will replace all elements with the given name.
    pub fn replacend(&mut self, handler: &Handler) {
        if self.has(handler.name()) {
            self.replace(handler);
        } else {
            self.append(vec![handler.to_owned()]);
        }
    }

    /// remove will return an updated handler with all instances of the specific
    /// named handler being removed
    pub fn remove(&mut self, name: TypeId) {
        let mut new_list: Vec<Handler> = Vec::new();
        for h in &self.0 {
            if h.name() != name {
                new_list.push(h.to_owned());
            }
        }
        self.0 = new_list;
    }

    /// clear clears all named handler in the list
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// run will execute each instruction in the handler list. If an error occurs in
    /// any of the handlers, then the list will halt execution and return the error.
    pub async fn run(&self, m: &mut Machine) -> Result<(), MachineError> {
        for handler in &self.0 {
            debug!("Running handler {:#?}", handler.name());
            handler.func(m).await.map_err(|e| {
                warn!("Failed handler {:#?}: {}", handler.name(), e.to_string());
                e
            })?;
        }

        Ok(())
    }
}

impl Default for Handlers {
    fn default() -> Self {
        Handlers {
            validation: HandlerList::default_validation_handler_list(),
            fcinit: HandlerList::default_fcinit_handler_list(),
        }
    }
}

impl Handlers {
    /// run will execute all handlers in the Handlers object by flattening the lists
    /// into a single list and running
    pub async fn run(&self, m: &mut Machine) -> Result<(), MachineError> {
        self.validation.run(m).await?;
        self.fcinit.run(m).await?;

        Ok(())
    }
}

// link_files_handler creates a new link files handler that will link files to
// the rootfs
// fn link_files_handler(kernel_image_file_name: PathBuf) -> Handler {
//     Handler {
//         name: LinkFilesToRootFSHandlerName,
//         func: Box::new(move |m: &mut Machine| -> Result<()> {
//             if m.cfg.jailer_cfg.is_none() {
//                 return Err("jailer config was not set for use".into());
//             }

//             // assemble the path to the jailed root folder on the host
//             let rootfs: PathBuf = [
//                 m.cfg
//                     .jailer_cfg
//                     .as_mut()
//                     .unwrap()
//                     .chroot_base_dir
//                     .to_owned()
//                     .unwrap_or(DEFAULT_JAILER_PATH.into()),
//                 m.cfg
//                     .jailer_cfg
//                     .as_mut()
//                     .unwrap()
//                     .exec_file
//                     .as_ref()
//                     .unwrap()
//                     .as_path()
//                     .file_name()
//                     .ok_or("malformed firecracker exec file name")?
//                     .into(),
//                 m.cfg
//                     .jailer_cfg
//                     .as_ref()
//                     .unwrap()
//                     .id
//                     .as_ref()
//                     .unwrap()
//                     .into(),
//                 ROOTFS_FOLDER_NAME.into(),
//             ]
//             .iter()
//             .collect();

//             // copy kernel image to root fs
//             std::fs::hard_link(
//                 &m.cfg.kernel_image_path.as_ref().unwrap(),
//                 [&rootfs, &kernel_image_file_name.to_owned().into()]
//                     .iter()
//                     .collect::<PathBuf>(),
//             )?;

//             let mut initrd_file_name: PathBuf = "".into();
//             if m.cfg.initrd_path.is_some()
//                 && m.cfg.initrd_path.to_owned().unwrap().as_os_str() != ""
//             {
//                 initrd_file_name = m
//                     .cfg
//                     .initrd_path
//                     .to_owned()
//                     .unwrap()
//                     .as_path()
//                     .file_name()
//                     .ok_or("malformed initrd path")?
//                     .into();
//                 std::fs::hard_link(
//                     &m.cfg.initrd_path.as_mut().unwrap(),
//                     [&rootfs, &initrd_file_name].iter().collect::<PathBuf>(),
//                 )?;
//             }

//             // copy all drives to the root fs
//             for drive in m.cfg.drives.as_mut().unwrap() {
//                 let host_path = &drive.get_path_on_host();
//                 let drive_file_name: PathBuf = host_path
//                     .as_path()
//                     .file_name()
//                     .ok_or("malformed drive file name")?
//                     .into();

//                 std::fs::hard_link(
//                     host_path,
//                     [&rootfs, &drive_file_name].iter().collect::<PathBuf>(),
//                 )?;
//                 // drive.path_on_host = drive_file_name;
//                 drive.set_drive_path(drive_file_name);
//             }

//             m.cfg.kernel_image_path = kernel_image_file_name.to_owned().into();
//             if m.cfg.initrd_path.is_some() && m.cfg.initrd_path.as_mut().unwrap().as_os_str() != ""
//             {
//                 m.cfg.initrd_path = Some(initrd_file_name);
//             }

//             for fifo_path in [&mut m.cfg.log_fifo, &mut m.cfg.metrics_fifo] {
//                 if fifo_path.is_none() || fifo_path.as_ref().unwrap().as_os_str() == "" {
//                     continue;
//                 }

//                 let file_name: PathBuf = fifo_path
//                     .as_mut()
//                     .unwrap()
//                     .as_path()
//                     .file_name()
//                     .ok_or("malformed fifo path")?
//                     .into();
//                 std::fs::hard_link(
//                     fifo_path.as_mut().unwrap(),
//                     [&rootfs, &file_name].iter().collect::<PathBuf>(),
//                 )?;

//                 nix::unistd::chown(
//                     &[&rootfs, &file_name].iter().collect::<PathBuf>(),
//                     Some(nix::unistd::Uid::from_raw(
//                         *m.cfg.jailer_cfg.as_ref().unwrap().uid.as_ref().unwrap() as u32,
//                     )),
//                     Some(nix::unistd::Gid::from_raw(
//                         *m.cfg.jailer_cfg.as_ref().unwrap().gid.as_ref().unwrap() as u32,
//                     )),
//                 )?;

//                 // update fifoPath as jailer works relative to the chroot dir
//                 *fifo_path = Some(file_name);
//             }

//             Ok(())
//         }),
//     }
// }
