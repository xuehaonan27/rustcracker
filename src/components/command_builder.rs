use std::path::PathBuf;

use crate::utils::DEFAULT_JAILER_BINARY;

pub const DEFAULT_FC_BIN: &'static str = "firecracker";
pub struct VMMCommandBuilder {
    bin: Option<PathBuf>,
    args: Option<Vec<String>>,
    socket_path: Option<PathBuf>,
    stdin: Option<std::process::Stdio>,
    stdout: Option<std::process::Stdio>,
    stderr: Option<std::process::Stdio>,
}

impl Default for VMMCommandBuilder {
    fn default() -> Self {
        Self::new()
            .with_bin(&DEFAULT_FC_BIN.into())
            .with_stdin(std::process::Stdio::inherit())
            .with_stdout(std::process::Stdio::inherit())
            .with_stderr(std::process::Stdio::inherit())
    }
}

impl VMMCommandBuilder {
    /// new returns a blank Builder with all fields set to None
    pub fn new() -> Self {
        Self {
            bin: None,
            args: None,
            socket_path: None,
            stdin: None,
            stdout: None,
            stderr: None,
        }
    }

    /// with_args specifies with arguments to pass through to the
    /// firecracker Command
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = Some(args);
        self
    }

    /// add_args will append the provided args to the given Command
    pub fn add_args(&mut self, mut args: Vec<String>) {
        if self.args.is_none() {
            self.args = Some(args);
        } else {
            self.args.as_mut().unwrap().append(&mut args);
        }
    }

    /// bin returns the bin that was set. If bin had not been set, then the default value
    /// will be returned
    pub fn bin(&self) -> PathBuf {
        if self.bin.is_none() {
            DEFAULT_FC_BIN.into()
        } else {
            self.bin.to_owned().unwrap()
        }
    }

    /// with_bin specifies which binary for firecrakcer to use
    pub fn with_bin(mut self, bin: &PathBuf) -> Self {
        self.bin = Some(bin.to_path_buf());
        self
    }

    /// with_socket_path specifies the socket path to be used when
    /// creating the firecracker Command
    pub fn with_socket_path(mut self, path: &PathBuf) -> Self {
        self.socket_path = Some(path.to_owned());
        self
    }

    /// with_stdin specifies which io.Reader to use in place of the os.Stdin in the
    /// firecracker exec.Command.
    pub fn with_stdin(mut self, stdin: impl Into<std::process::Stdio>) -> Self {
        self.stdin = Some(stdin.into());
        self
    }

    /// with_stdout specifies which io.Writer to use in place of the os.Stdout in the
    /// firecracker exec.Command.
    pub fn with_stdout(mut self, stdout: impl Into<std::process::Stdio>) -> Self {
        self.stdout = Some(stdout.into());
        self
    }

    /// with_stderr specifies which io.Writer to use in place of the os.Stderr in the
    /// firecracker exec.Command.
    pub fn with_stderr(mut self, stderr: impl Into<std::process::Stdio>) -> Self {
        self.stderr = Some(stderr.into());
        self
    }

    /// build will build a firecracker command using the specific arguments
    /// specified in the builder.
    pub fn build(self) -> std::process::Command {
        let mut cmd = std::process::Command::new(self.bin());
        if self.socket_path.is_some() {
            cmd.arg("--api-sock");
            cmd.arg(self.socket_path.as_ref().unwrap());
        }
        if self.args.is_some() {
            cmd.args(self.args.as_ref().unwrap());
        }
        if let Some(stdout) = self.stdout {
            cmd.stdout(stdout);
        }
        if let Some(stderr) = self.stderr {
            cmd.stderr(stderr);
        }
        if let Some(stdin) = self.stdin {
            cmd.stdin(stdin);
        }

        cmd
    }
}


pub struct JailerCommandBuilder {
    bin: PathBuf,
    id: String,
    uid: u32,
    gid: u32,
    exec_file: PathBuf,
    node: usize,

    // optional params
    chroot_base_dir: Option<PathBuf>,
    net_ns: Option<PathBuf>,
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
            bin: DEFAULT_JAILER_BINARY.into(),
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
            // "--node".into(),
            // self.node.to_string(),
        ];

        if let Some(chroot_base_dir) = &self.chroot_base_dir {
            args.push("--chroot-base-dir".into());
            args.push(chroot_base_dir.to_string_lossy().to_string());
        }

        if let Some(net_ns) = &self.net_ns {
            args.push("--netns".into());
            args.push(net_ns.to_string_lossy().to_string());
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
    pub fn with_uid(mut self, uid: &u32) -> Self {
        self.uid = *uid;
        self
    }

    // with_gid will set the specified gid to the builder.
    pub fn with_gid(mut self, gid: &u32) -> Self {
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
    pub fn with_net_ns(mut self, path: impl Into<PathBuf>) -> Self {
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
        cmd.args(self.args());
        if let Some(stdin) = self.stdin {
            cmd.stdin(stdin);
        }
        if let Some(stdout) = self.stdout {
            cmd.stdout(stdout);
        }
        if let Some(stderr) = self.stderr {
            cmd.stderr(stderr);
        }
        // std::mem::replace(cmd, std::process::Command::new(""))
        cmd
    }
}
