use crate::config::HypervisorConfig;
use crate::RtckResult;

pub struct HypervisorOptions {
    config: HypervisorConfig,
}

impl HypervisorOptions {
    pub fn new() -> Self {
        Self {
            config: Default::default(),
        }
    }

    pub fn validate(self) -> RtckResult<Self> {
        self.config.validate()?;
        Ok(self)
    }

    pub async fn spawn(&self) -> RtckResult<crate::hypervisor::Hypervisor> {
        use crate::hypervisor::Hypervisor;
        Hypervisor::new(&self.config).await
    }

    pub fn spawn_sync(&self) -> RtckResult<crate::sync_hypervisor::Hypervisor> {
        use crate::sync_hypervisor::Hypervisor;
        Hypervisor::new(&self.config)
    }

    /// Instance id.
    /// If set to None, then the hypervisor will allocate a random one for you.
    /// So if you want to make sure which hypervisor you are running now,
    /// you'd better assign a value to this field :)
    pub fn id(mut self, id: &String) -> Self {
        self.config.id = Some(id.clone());
        self
    }

    /// Launch timeout
    pub fn launch_timeout(mut self, timeout: u64) -> Self {
        self.config.launch_timeout = timeout;
        self
    }

    /// Interval in seconds for hypervisor polling the status
    /// of the microVM it holds when waiting for the user to
    /// give up the microVM.
    /// Default to 10 (seconds).
    /// Could be set bigger to avoid large amount of log being produced.
    pub fn poll_status_secs(mut self, interval: u64) -> Self {
        self.config.poll_status_secs = interval;
        self
    }

    /// Whether jailer should be used or not
    pub fn using_jailer(mut self, b: bool) -> Self {
        self.config.using_jailer = Some(b);
        if self.config.jailer_config == None {
            self.config.jailer_config = Some(Default::default());
            // unwrap safe
            self.config.jailer_config().unwrap().jailer_bin = self.config.jailer_bin.clone();
        }
        self
    }

    /// Path to firecracker binary
    pub fn frck_bin<P: AsRef<str>>(mut self, path: P) -> Self {
        self.config.frck_bin = Some(path.as_ref().into());
        self
    }

    /// Path to jailer binary (if using jailer)
    pub fn jailer_bin<P: AsRef<str>>(mut self, path: P) -> Self {
        self.config.frck_bin = Some(path.as_ref().into());
        self
    }

    /// Where to put firecracker exported config for `--config`
    pub fn frck_export_path<P: AsRef<str>>(mut self, path: P) -> Self {
        self.config.frck_export_path = Some(path.as_ref().into());
        self
    }

    /// Where to put socket, default to None, and hypervisor will allocate one for you.
    /// When using jailer, default value "run/firecracker.socket" is used since this
    /// name is impossible to conflict with other sockets when using jailer.
    /// When not using jailer, default value would be "/run/firecracker-<id>.socket",
    /// where <id> is the instance id of your hypervisor.
    pub fn socket_path<P: AsRef<str>>(mut self, path: P) -> Self {
        self.config.socket_path = Some(path.as_ref().into());
        self
    }

    /// Socket retrying times, default to 3 times
    pub fn socket_retry(mut self, retry: usize) -> Self {
        self.config.socket_retry = retry;
        self
    }

    /// Where to put lock file, default to None, and Local will allocate one for you
    pub fn lock_path<P: AsRef<str>>(mut self, path: P) -> Self {
        self.config.lock_path = Some(path.as_ref().into());
        self
    }

    /// Hypervisor log path
    /// path inside jailer seen by firecracker (when using jailer)
    pub fn log_path<P: AsRef<str>>(mut self, path: P) -> Self {
        self.config.log_path = Some(path.as_ref().into());
        self
    }

    /// Whether log files should be removed after microVM
    /// instance is removed. Default to false.
    pub fn log_clear(mut self, b: bool) -> Self {
        self.config.log_clear = Some(b);
        self
    }

    /// Hypervisor metrics path
    /// path inside jailer seen by firecracker (when using jailer)
    pub fn metrics_path<P: AsRef<str>>(mut self, path: P) -> Self {
        self.config.metrics_path = Some(path.as_ref().into());
        self
    }

    /// Whether metrics files should be removed after microVM
    /// instance is removed. Default to false.
    pub fn metrics_clear(mut self, b: bool) -> Self {
        self.config.metrics_clear = Some(b);
        self
    }

    /// Whether networking should be cleared after microVM
    /// instance is removed. Default to false.
    pub fn network_clear(mut self, b: bool) -> Self {
        self.config.network_clear = Some(b);
        self
    }

    /// Whether seccomp filters should be installed and how restrictive
    /// they should be. Possible values are:
    ///
    ///	0 : (default): disabled.
    ///	1 : basic filtering. This prohibits syscalls not whitelisted by Firecracker.
    ///	2 : advanced filtering. This adds further checks on some of the
    ///			parameters of the allowed syscalls.
    pub fn seccmop_level(mut self, level: usize) -> Self {
        self.config.seccomp_level = Some(level);
        self
    }

    /// Whether jailer working directory should be removed after
    /// microVM instance is removed, default to false
    pub fn clear_jailer(mut self, b: bool) -> Self {
        self.config.clear_jailer = Some(b);
        self
    }

    /// `gid` the jailer switches to as it execs the target binary.
    pub fn jailer_gid(mut self, gid: u32) -> Self {
        if let Some(c) = self.config.jailer_config() {
            c.gid = Some(gid);
        }
        self
    }

    /// `uid` the jailer switches to as it execs the target binary.
    pub fn jailer_uid(mut self, uid: u32) -> Self {
        if let Some(c) = self.config.jailer_config() {
            c.uid = Some(uid);
        }
        self
    }

    /// `numa_node` represents the NUMA node the process gets assigned to.
    pub fn numa_node(mut self, node: usize) -> Self {
        if let Some(c) = self.config.jailer_config() {
            c.numa_node = Some(node);
        }
        self
    }

    /// `exec_file` is the path to the Firecracker binary that will be exec-ed by
    /// the jailer. The user can provide a path to any binary, but the interaction
    /// with the jailer is mostly Firecracker specific.
    pub fn exec_file<P: AsRef<str>>(mut self, path: P) -> Self {
        if let Some(c) = self.config.jailer_config() {
            c.exec_file = Some(path.as_ref().into());
        }
        self
    }

    /// `chroot_base_dir` represents the base folder where chroot jails are built. The
    /// default is /srv/jailer
    pub fn chroot_base_dir<P: AsRef<str>>(mut self, path: P) -> Self {
        if let Some(c) = self.config.jailer_config() {
            c.chroot_base_dir = Some(path.as_ref().into());
        }
        self
    }

    /// `daemonize` is set to true, call setsid() and redirect STDIN, STDOUT, and
    /// STDERR to /dev/null
    pub fn daemonize(mut self, b: bool) -> Self {
        if let Some(c) = self.config.jailer_config() {
            c.daemonize = Some(b);
        }
        self
    }
}
