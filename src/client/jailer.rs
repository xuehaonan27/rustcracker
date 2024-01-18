use std::{
    os::{fd::FromRawFd, unix::fs::OpenOptionsExt},
    path::PathBuf,
};

use log::error;
use serde::{Deserialize, Serialize};

use super::{
    handler::HandlersAdapter,
    machine::{Config, Machine, MachineError},
};

pub const DEFAULT_JAILER_PATH: &'static str = "/srv/jailer";
const DEFAULT_JAILER_BIN: &'static str = "jailer";
pub const ROOTFS_FOLDER_NAME: &'static str = "root";
const DEFAULT_SOCKET_PATH: &'static str = "/run/firecracker.socket";

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
    pub(super) gid: Option<usize>,

    // UID the jailer switches to as it execs the target binary.
    pub(super) uid: Option<usize>,

    // ID is the unique VM identification string, which may contain alphanumeric
    // characters and hyphens. The maximum id length is currently 64 characters
    pub(super) id: Option<String>,

    // NumaNode represents the NUMA node the process gets assigned to.
    pub(super) numa_node: Option<usize>,

    // ExecFile is the path to the Firecracker binary that will be exec-ed by
    // the jailer. The user can provide a path to any binary, but the interaction
    // with the jailer is mostly Firecracker specific.
    pub(super) exec_file: Option<PathBuf>,

    // JailerBinary specifies the jailer binary to be used for setting up the
    // Firecracker VM jail. If the value contains no path separators, it will
    // use the PATH environment variable to get the absolute path of the binary.
    // If the value contains path separators, the value will be used directly
    // to exec the jailer. This follows the same conventions as Golang's
    // os/exec.Command.
    //
    // If not specified it defaults to "jailer".
    pub(super) jailer_binary: Option<PathBuf>,

    // ChrootBaseDir represents the base folder where chroot jails are built. The
    // default is /srv/jailer
    pub(super) chroot_base_dir: Option<PathBuf>,

    //  Daemonize is set to true, call setsid() and redirect STDIN, STDOUT, and
    //  STDERR to /dev/null
    pub(super) daemonize: Option<bool>,

    // ChrootStrategy will dictate how files are transfered to the root drive.
    pub(super) chroot_strategy: Option<HandlersAdapter>,

    // Stdout specifies the IO writer for STDOUT to use when spawning the jailer.
    // pub(crate) stdout: Option<std::process::Stdio>,
    pub(super) stdout: Option<StdioTypes>,

    // Stderr specifies the IO writer for STDERR to use when spawning the jailer.
    pub(super) stderr: Option<StdioTypes>,

    // Stdin specifies the IO reader for STDIN to use when spawning the jailer.
    pub(super) stdin: Option<StdioTypes>,
}

pub struct JailerCommandBuilder {
    bin: PathBuf,
    id: String,
    uid: usize,
    gid: usize,
    exec_file: PathBuf,
    node: usize,

    // optional params
    chroot_base_dir: Option<PathBuf>,
    net_ns: Option<String>,
    daemonize: Option<bool>,
    fircracker_args: Option<Vec<String>>,

    stdin: Option<std::process::Stdio>,
    stdout: Option<std::process::Stdio>,
    stderr: Option<std::process::Stdio>,
}

impl JailerCommandBuilder {
    // new will return a new jailer command builder with the
    // proper default value initialized.
    pub fn new() -> Self {
        Self {
            bin: DEFAULT_JAILER_BIN.into(),
            id: "".into(),
            uid: 0,
            gid: 0,
            exec_file: "".into(),
            node: 0,
            chroot_base_dir: None,
            net_ns: None,
            daemonize: None,
            fircracker_args: None,
            stdin: None,
            stdout: None,
            stderr: None,
        }
    }

    // args returns the specified set of args to be used
    // in command construction.
    pub fn args(&self) -> Vec<String> {
        let mut args: Vec<String> = vec![
            "--id".into(),
            self.id.clone(),
            "--uid".into(),
            self.uid.to_string(),
            "--gid".into(),
            self.gid.to_string(),
            "--exec-file".into(),
            self.exec_file.to_string_lossy().to_string(),
            "--node".into(),
            self.node.to_string(),
        ];

        if let Some(chroot_base_dir) = &self.chroot_base_dir {
            args.push("--chroot-base-dir".into());
            args.push(chroot_base_dir.to_string_lossy().to_string());
        }

        if let Some(net_ns) = &self.net_ns {
            args.push("--netns".into());
            args.push(net_ns.to_string());
        }

        if let Some(true) = self.daemonize {
            args.push("--daemonize".into());
        }

        if let Some(mut firecracker_args) = self.fircracker_args.clone() {
            args.push("--".into());
            args.append(&mut firecracker_args);
        }

        args
    }

    // bin returns the jailer bin path. If bin path is empty, then the default path
    // will be returned.
    pub fn bin(&self) -> PathBuf {
        self.bin.clone()
    }

    // with_bin will set the specific bin path to the builder.
    pub fn with_bin(mut self, bin: impl Into<PathBuf>) -> Self {
        self.bin = bin.into();
        self
    }

    // with_id will set the specified id to the builder.
    pub fn with_id(mut self, id: &String) -> Self {
        self.id = id.to_owned();
        self
    }

    // with_uid will set the specified uid to the builder.
    pub fn with_uid(mut self, uid: &usize) -> Self {
        self.uid = *uid;
        self
    }

    // with_gid will set the specified gid to the builder.
    pub fn with_gid(mut self, gid: &usize) -> Self {
        self.gid = *gid;
        self
    }

    // with_exec_file will set the specified path to the builder. This represents a
    // firecracker binary used when calling the jailer.
    pub fn with_exec_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.exec_file = path.into();
        self
    }

    // with_numa_node uses the specfied node for the jailer. This represents the numa
    // node that the process will get assigned to.
    pub fn with_numa_node(mut self, node: &usize) -> Self {
        self.node = *node;
        self
    }

    // with_chroot_base_dir will set the given path as the chroot base directory. This
    // specifies where chroot jails are built and defaults to /srv/jailer.
    pub fn with_chroot_base_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.chroot_base_dir = Some(path.into());
        self
    }

    // with_net_ns will set the given path to the net namespace of the builder. This
    // represents the path to a network namespace handle and will be used to join
    // the associated network namepsace.
    pub fn with_net_ns(mut self, path: impl Into<String>) -> Self {
        self.net_ns = Some(path.into());
        self
    }

    // with_daemonize will specify whether to set stdio to /dev/null
    pub fn with_daemonize(mut self, daemonize: &bool) -> Self {
        self.daemonize = Some(*daemonize);
        self
    }

    // stdin will return the stdin that will be used when creating the firecracker
    // exec.Command
    // pub fn stdin(&self) -> Option<std::process::Stdio> {
    //     self.stdin
    // }

    // with_stdin specifies which io.Reader to use in place of the os.Stdin in the
    // firecracker exec.Command.
    pub fn with_stdin(mut self, stdin: impl Into<std::process::Stdio>) -> Self {
        self.stdin = Some(stdin.into());
        self
    }

    // stdout will return the stdout that will be used when creating the
    // firecracker exec.Command
    // pub fn stdout(&self) -> Option<std::process::Stdio> {
    //     self.stdout
    // }

    // with_stdout specifies which io.Writer to use in place of the os.Stdout in the
    // firecracker exec.Command.
    pub fn with_stdout(mut self, stdout: impl Into<std::process::Stdio>) -> Self {
        self.stdout = Some(stdout.into());
        self
    }

    // stderr will return the stderr that will be used when creating the
    // firecracker exec.command
    // pub fn stderr(&self) -> Option<std::process::Stdio> {
    //     self.stderr
    // }

    // with_stderr specifies which io.Writer to use in place of the os.Stderr in the
    // firecracker exec.Command.
    pub fn with_stderr(mut self, stderr: impl Into<std::process::Stdio>) -> Self {
        self.stderr = Some(stderr.into());
        self
    }

    // with_firecracker_args will adds these arguments to the end of the argument
    // chain which the jailer will intepret to belonging to Firecracke
    pub fn with_firecracker_args(mut self, args: impl Into<Vec<String>>) -> Self {
        self.fircracker_args = Some(args.into());
        self
    }

    pub fn build(self) -> std::process::Command {
        let mut cmd = std::process::Command::new(&self.bin);
        let cmd = cmd.args(self.args());
        if let Some(stdin) = self.stdin {
            cmd.stdin(stdin);
        }
        if let Some(stdout) = self.stdout {
            cmd.stdout(stdout);
        }
        if let Some(stderr) = self.stderr {
            cmd.stderr(stderr);
        }
        std::mem::replace(cmd, std::process::Command::new(""))
    }
}

/// jail will set up proper handlers and remove configuration validation due to
/// stating of files
pub fn jail(m: &mut Machine, mut cfg: Config) -> Result<(), MachineError> {
    let machine_socket_path: PathBuf;
    if let Some(socket_path) = cfg.socket_path {
        machine_socket_path = socket_path;
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
                    .ok_or(MachineError::FileError(
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
                    .ok_or(MachineError::FileError(
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
                            MachineError::FileError(format!(
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
                            MachineError::FileError(format!(
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
                "--seccomp-level".to_string(),
                cfg.seccomp_level.unwrap().to_string(),
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
                            MachineError::FileError(format!(
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

        if jailer_cfg.chroot_strategy.is_none() {
            error!("chroot strategy was not set for use");
            return Err(MachineError::Initialize(
                "chroot strategy was not set for use".to_string(),
            ));
        } else {
            jailer_cfg
                .chroot_strategy
                .as_ref()
                .unwrap()
                .adapt_handlers(&mut m.handlers)?;
        }

        Ok(())
    } else {
        Err(MachineError::Initialize(
            "jailer config was not set for use".to_string(),
        ))
    }
}
