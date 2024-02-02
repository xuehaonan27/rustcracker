use std::{
    os::{fd::FromRawFd, unix::fs::OpenOptionsExt},
    path::PathBuf,
};

use log::error;
use serde::{Deserialize, Serialize};

use crate::utils::{DEFAULT_JAILER_PATH, DEFAULT_SOCKET_PATH, ROOTFS_FOLDER_NAME};

use super::{command_builder::JailerCommandBuilder, machine::{Config, Machine, MachineError}};

/*
let from = std::process::Stdio::from(value);
let null = std::process::Stdio::null();
let inherit = std::process::Stdio::inherit();
let piped = std::process::Stdio::piped();
let from_raw_fd = std::process::Stdio::from_raw_fd(fd);
*/

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum StdioTypes {
    // 空设备
    Null,
    // 进程管道的
    Piped,
    // 继承进程的
    Inherit,
    // 从指定文件打开的, std::fs::File
    From { path: PathBuf },
    // 从指定文件描述符打开的
    FromRawFd { fd: i32 },
}

impl StdioTypes {
    pub fn open_io(&self) -> std::io::Result<std::process::Stdio> {
        match self {
            StdioTypes::Null => Ok(std::process::Stdio::null()),
            StdioTypes::Piped => Ok(std::process::Stdio::piped()),
            StdioTypes::Inherit => Ok(std::process::Stdio::inherit()),
            StdioTypes::From { path } => Ok(std::process::Stdio::from({
                let mut options = std::fs::OpenOptions::new();
                options.mode(0o644);
                options.open(&path)?
            })),
            StdioTypes::FromRawFd { fd } => {
                Ok(unsafe { std::process::Stdio::from_raw_fd(fd.to_owned()) })
            }
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct JailerConfig {
    // GID the jailer switches to as it execs the target binary.
    pub gid: Option<u32>,

    // UID the jailer switches to as it execs the target binary.
    pub uid: Option<u32>,

    // ID is the unique VM identification string, which may contain alphanumeric
    // characters and hyphens. The maximum id length is currently 64 characters
    pub id: Option<String>,

    // NumaNode represents the NUMA node the process gets assigned to.
    pub numa_node: Option<usize>,

    // ExecFile is the path to the Firecracker binary that will be exec-ed by
    // the jailer. The user can provide a path to any binary, but the interaction
    // with the jailer is mostly Firecracker specific.
    pub exec_file: Option<PathBuf>,

    // JailerBinary specifies the jailer binary to be used for setting up the
    // Firecracker VM jail.
    // If not specified it defaults to "jailer".
    pub jailer_binary: Option<PathBuf>,

    // ChrootBaseDir represents the base folder where chroot jails are built. The
    // default is /srv/jailer
    pub chroot_base_dir: Option<PathBuf>,

    //  Daemonize is set to true, call setsid() and redirect STDIN, STDOUT, and
    //  STDERR to /dev/null
    pub daemonize: Option<bool>,

    // ChrootStrategy will dictate how files are transfered to the root drive.
    // pub chroot_strategy: Option<HandlersAdapter>,

    // Stdout specifies the IO writer for STDOUT to use when spawning the jailer.
    // pub(crate) stdout: Option<std::process::Stdio>,
    pub stdout: Option<StdioTypes>,

    // Stderr specifies the IO writer for STDERR to use when spawning the jailer.
    pub stderr: Option<StdioTypes>,

    // Stdin specifies the IO reader for STDIN to use when spawning the jailer.
    pub stdin: Option<StdioTypes>,
}

// impl JailerConfig {
//     /// called by JailerConfigValidationHandler
//     pub(super) fn validate(&self) -> Result<(), MachineError> {
//         if self.exec_file.is_none() {
//             error!(target: "JailerConfig::validate", "should assign firecracker binary");
//             return Err(MachineError::Validation(format!(
//                 "should assign firecracker binary"
//             )));
//         } else if let Some(path) = self.exec_file.as_ref() {
//             if let Err(e) = std::fs::metadata(path) {
//                 error!(target: "JailerConfig::validate", "fail to stat firecracker binary {}: {}", path.display(), e.to_string());
//                 return Err(MachineError::Validation(format!(
//                     "fail to stat firecracker binary {}: {}", path.display(), e.to_string()
//                 )));
//             }
//         }

//         let mut path: &PathBuf = &"jailer".into();
//         if self.jailer_binary.is_none() {
//             info!(target: "JailerConfig::validate", "no jailer binary specified, using \"jailer\"")
//         } else {
//             path = self.jailer_binary.as_ref().unwrap();
//         }
//         if let Err(e) = std::fs::metadata(path) {
//             error!(target: "JailerConfig::validate", "fail to stat jailer binary {}: {}", path.display(), e.to_string());
//             return Err(MachineError::Validation(format!(
//                 "fail to stat jailer binary {}: {}", path.display(), e.to_string()
//             )));
//         }

//         let mut path: &PathBuf = &"/srv/jailer".into();
//         if self.chroot_base_dir.is_none() {
//             info!(target: "JailerConfig::validate", "no chroot base directory specified, using \"/srv/jailer\"")
//         } else {
//             path = self.chroot_base_dir.as_ref().unwrap();
//         }
//         if let Err(e) = std::fs::metadata(path) {
//             error!(target: "JailerConfig::validate", "fail to stat chroot base directory {}: {}", path.display(), e.to_string());
//             return Err(MachineError::Validation(format!(
//                 "fail to stat chroot base directory {}: {}", path.display(), e.to_string()
//             )));
//         }

//         Ok(())
//     }
// }

/// jail will set up proper handlers and remove configuration validation due to
/// stating of files
pub fn jail(m: &mut Machine, cfg: &mut Config) -> Result<(), MachineError> {
    // assemble machine socket path
    let machine_socket_path: PathBuf;
    if let Some(socket_path) = &cfg.socket_path {
        machine_socket_path = socket_path.to_path_buf();
    } else {
        machine_socket_path = DEFAULT_SOCKET_PATH.into();
    }

    let jailer_workspace_dir: PathBuf;
    if let Some(jailer_cfg) = &cfg.jailer_cfg {
        if let Some(chroot_base_dir) = &jailer_cfg.chroot_base_dir {
            jailer_workspace_dir = [
                chroot_base_dir.to_owned(),
                jailer_cfg
                    .exec_file
                    .as_ref()
                    .unwrap()
                    .as_path()
                    .file_name()
                    .ok_or(MachineError::ArgWrong(
                        "malformed firecracker exec file name".to_string(),
                    ))?
                    .into(),
                jailer_cfg.id.as_ref().unwrap().into(),
                ROOTFS_FOLDER_NAME.into(),
            ]
            .iter()
            .collect();
        } else {
            jailer_workspace_dir = [
                PathBuf::from(DEFAULT_JAILER_PATH),
                jailer_cfg
                    .exec_file
                    .as_ref()
                    .unwrap()
                    .as_path()
                    .file_name()
                    .ok_or(MachineError::ArgWrong(
                        "malformed firecracker exec file name".to_string(),
                    ))?
                    .into(),
                jailer_cfg.id.as_ref().unwrap().into(),
                ROOTFS_FOLDER_NAME.into(),
            ]
            .iter()
            .collect();
        }

        cfg.socket_path = Some(jailer_workspace_dir.join(&machine_socket_path));

        let mut stdout = std::process::Stdio::inherit();
        if jailer_cfg.stdout.is_some() {
            // stdout = jailer_cfg.stdout.unwrap();
            match &jailer_cfg.stdout {
                None => (),
                Some(StdioTypes::Null) => stdout = std::process::Stdio::null(),
                Some(StdioTypes::Piped) => stdout = std::process::Stdio::piped(),
                Some(StdioTypes::Inherit) => stdout = std::process::Stdio::inherit(),
                Some(StdioTypes::From { path }) => {
                    stdout = std::process::Stdio::from({
                        let mut options = std::fs::OpenOptions::new();
                        options.mode(0o644);
                        options.open(&path).map_err(|e| {
                            error!("fail to open file at {}: {}", path.display(), e.to_string());
                            MachineError::FileAccess(format!(
                                "fail to open file at {}: {}",
                                path.display(),
                                e.to_string()
                            ))
                        })?
                    })
                }
                Some(StdioTypes::FromRawFd { fd }) => {
                    stdout = unsafe { std::process::Stdio::from_raw_fd(fd.to_owned()) }
                }
            }
        }

        let mut stderr = std::process::Stdio::inherit();
        if jailer_cfg.stderr.is_some() {
            // stderr = jailer_cfg.stderr.unwrap();
            match &jailer_cfg.stdout {
                None => (),
                Some(StdioTypes::Null) => stderr = std::process::Stdio::null(),
                Some(StdioTypes::Piped) => stderr = std::process::Stdio::piped(),
                Some(StdioTypes::Inherit) => stderr = std::process::Stdio::inherit(),
                Some(StdioTypes::From { path }) => {
                    stderr = std::process::Stdio::from({
                        let mut options = std::fs::OpenOptions::new();
                        options.mode(0o644);
                        options.open(&path).map_err(|e| {
                            error!("fail to open file at {}: {}", path.display(), e.to_string());
                            MachineError::FileAccess(format!(
                                "fail to open file at {}: {}",
                                path.display(),
                                e.to_string()
                            ))
                        })?
                    })
                }
                Some(StdioTypes::FromRawFd { fd }) => {
                    stderr = unsafe { std::process::Stdio::from_raw_fd(fd.to_owned()) }
                }
            }
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
                // "--seccomp-level".to_string(),
                // cfg.seccomp_level.unwrap().to_string(),
                "--api-sock".to_string(),
                machine_socket_path.to_string_lossy().to_string(),
            ])
            .with_stdout(stdout)
            .with_stderr(stderr);

        if let Some(jailer_binary) = &jailer_cfg.jailer_binary {
            builder = builder.with_bin(jailer_binary);
        }

        if let Some(net_ns) = &cfg.net_ns {
            builder = builder.with_net_ns(net_ns);
        }

        let mut stdin = std::process::Stdio::inherit();
        if jailer_cfg.stdin.is_some() {
            // stdin = jailer_cfg.stdin.unwrap();
            match &jailer_cfg.stdout {
                None => (),
                Some(StdioTypes::Null) => stdin = std::process::Stdio::null(),
                Some(StdioTypes::Piped) => stdin = std::process::Stdio::piped(),
                Some(StdioTypes::Inherit) => stdin = std::process::Stdio::inherit(),
                Some(StdioTypes::From { path }) => {
                    stdin = std::process::Stdio::from({
                        let mut options = std::fs::OpenOptions::new();
                        options.mode(0o644);
                        options.open(&path).map_err(|e| {
                            error!("fail to open file at {}: {}", path.display(), e.to_string());
                            MachineError::FileAccess(format!(
                                "fail to open file at {}: {}",
                                path.display(),
                                e.to_string()
                            ))
                        })?
                    })
                }
                Some(StdioTypes::FromRawFd { fd }) => {
                    stdin = unsafe { std::process::Stdio::from_raw_fd(fd.to_owned()) }
                }
            }
        }

        builder = builder.with_stdin(stdin);

        m.cmd = Some(builder.build().into());


        Ok(())
    } else {
        Err(MachineError::Initialize(
            "jailer config was not set for use".to_string(),
        ))
    }
}
