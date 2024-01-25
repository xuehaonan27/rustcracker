use std::path::PathBuf;

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
