use std::net::Ipv4Addr;

use crate::config::{HypervisorConfig, JailerConfig, MicroVMConfig};
use crate::models::*;
use crate::RtckResult;
use log::warn;

#[derive(Debug, Clone)]
pub struct HypervisorOptions {
    config: HypervisorConfig,
}

impl HypervisorOptions {
    pub fn new() -> Self {
        let jailer_config: JailerConfig = Default::default();
        let mut config: HypervisorConfig = Default::default();
        config.jailer_config = Some(jailer_config);
        Self { config }
    }

    pub fn validate(self) -> RtckResult<Self> {
        self.config.validate()?;
        Ok(self)
    }

    pub async fn spawn_async(&self) -> RtckResult<crate::Hypervisor> {
        use crate::Hypervisor;
        Hypervisor::new(&self.config).await
    }

    pub fn spawn_sync(&self) -> RtckResult<crate::HypervisorSync> {
        use crate::HypervisorSync;
        HypervisorSync::new(&self.config)
    }

    /// Instance id.
    /// If set to None, then the hypervisor will allocate a random one for you.
    /// So if you want to make sure which hypervisor you are running now,
    /// you'd better assign a value to this field :)
    pub fn id<S: AsRef<str>>(mut self, id: S) -> Self {
        self.config.id = Some(id.as_ref().into());
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
        self
    }

    /// Path to firecracker binary
    pub fn frck_bin<P: AsRef<str>>(mut self, path: P) -> Self {
        self.config.frck_bin = Some(path.as_ref().into());
        if let Some(c) = self.config.jailer_config() {
            c.exec_file = Some(path.as_ref().into());
        }
        self
    }

    /// Path to jailer binary (if using jailer)
    pub fn jailer_bin<P: AsRef<str>>(mut self, path: P) -> Self {
        self.config.jailer_bin = Some(path.as_ref().into());
        if let Some(c) = self.config.jailer_config() {
            c.jailer_bin = Some(path.as_ref().into());
        }
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
        if let Some(true) = self.config.using_jailer {
            let c = self.config.jailer_config().unwrap();
            c.gid = Some(gid);
        } else {
            warn!("Jailer not enabled, ignoring setting `jailer_gid = {gid}`");
        }
        self
    }

    /// `uid` the jailer switches to as it execs the target binary.
    pub fn jailer_uid(mut self, uid: u32) -> Self {
        if let Some(true) = self.config.using_jailer {
            let c = self.config.jailer_config().unwrap();
            c.uid = Some(uid);
        } else {
            warn!("Jailer not enabled, ignoring setting `jailer_uid = {uid}`");
        }
        self
    }

    /// `numa_node` represents the NUMA node the process gets assigned to.
    pub fn numa_node(mut self, node: usize) -> Self {
        if let Some(true) = self.config.using_jailer {
            let c = self.config.jailer_config().unwrap();
            c.numa_node = Some(node);
        } else {
            warn!("Jailer not enabled, ignoring setting `numa_node = {node}`");
        }
        self
    }

    /// `chroot_base_dir` represents the base folder where chroot jails are built. The
    /// default is /srv/jailer
    pub fn chroot_base_dir<P: AsRef<str>>(mut self, path: P) -> Self {
        if let Some(true) = self.config.using_jailer {
            let c = self.config.jailer_config().unwrap();
            c.chroot_base_dir = Some(path.as_ref().into());
        } else {
            let path: &str = path.as_ref().into();
            warn!("Jailer not enabled, ignoring setting `chroot_base_dir = {path}`");
        }
        self
    }

    /// `daemonize` is set to true, call setsid() and redirect STDIN, STDOUT, and
    /// STDERR to /dev/null
    pub fn daemonize(mut self, b: bool) -> Self {
        if let Some(true) = self.config.using_jailer {
            let c = self.config.jailer_config().unwrap();
            c.daemonize = Some(b);
        } else {
            warn!("Jailer not enabled, ignoring setting `daemonize = {b}`")
        }
        self
    }
}

pub struct MicroVMOptions {
    config: MicroVMConfig,
}

impl MicroVMOptions {
    pub fn new() -> Self {
        Self {
            config: Default::default(),
        }
    }

    pub fn validate(self) -> RtckResult<Self> {
        self.config.validate()?;
        Ok(self)
    }

    pub fn config(self) -> MicroVMConfig {
        self.config.clone()
    }

    pub async fn instance_async(&self, hypervisor: &mut crate::Hypervisor) -> RtckResult<()> {
        hypervisor.start(&self.config).await
    }

    pub fn instance_sync(&self, hypervisor: &mut crate::HypervisorSync) -> RtckResult<()> {
        hypervisor.start(&self.config)
    }

    /// The logger for microVM.
    pub fn logger(mut self, logger: Logger) -> Self {
        self.config.logger = Some(logger);
        self
    }

    /// The file path where the Firecracker metrics is located.
    pub fn metrics(mut self, metrics: Metrics) -> Self {
        self.config.metrics = Some(metrics);
        self
    }

    /// Kernel image path, initrd path (optional) and kernel args.
    pub fn boot_source(mut self, boot_source: BootSource) -> Self {
        self.config.boot_source = Some(boot_source);
        self
    }

    /// Block devices that should be made available to the microVM.
    pub fn drives(mut self, drives: Vec<Drive>) -> Self {
        self.config.drives = Some(drives);
        self
    }

    /// Tap devices that should be made available to the microVM.
    pub fn network_interfaces(mut self, network_interfaces: Vec<NetworkInterface>) -> Self {
        self.config.network_interfaces = Some(network_interfaces);
        self
    }

    /// Vsock devices that should be made available to the microVM.
    pub fn vsock_devices(mut self, vsock_devices: Vec<Vsock>) -> Self {
        self.config.vsock_devices = Some(vsock_devices);
        self
    }

    /// CPU configuration of microVM.
    pub fn cpu_config(mut self, cpu_config: CPUConfig) -> Self {
        self.config.cpu_config = Some(cpu_config);
        self
    }

    /// Firecracker microVM process configuration.
    pub fn machine_config(mut self, machine_config: MachineConfiguration) -> Self {
        self.config.machine_config = Some(machine_config);
        self
    }

    /// (Optional) vmid is a unique identifier for this VM. It's set to a
    /// random uuid if not provided by the user. It's used to set Firecracker's instance ID.
    pub fn vmid<S: AsRef<str>>(mut self, vmid: S) -> Self {
        self.config.vmid = Some(vmid.as_ref().into());
        self
    }

    /// The path to a network namespace handle. If present, the
    /// application will use this to join the associated network namespace
    pub fn net_ns<S: AsRef<str>>(mut self, net_ns: S) -> Self {
        self.config.net_ns = Some(net_ns.as_ref().into());
        self
    }

    /// IPv4 address used by guest applications when issuing requests to MMDS.
    pub fn mmds_address(mut self, address: Ipv4Addr) -> Self {
        self.config.mmds_address = Some(address);
        self
    }

    /// Balloon device that is to be put to the machine.
    pub fn balloon(mut self, balloon: Balloon) -> Self {
        self.config.balloon = Some(balloon);
        self
    }

    /// The entropy device.
    pub fn entropy_device(mut self, entropy_device: EntropyDevice) -> Self {
        self.config.entropy_device = Some(entropy_device);
        self
    }

    /// Initial metadata that is to be assigned to the machine.
    pub fn init_metadata<S: AsRef<str>>(mut self, init_metadata: S) -> Self {
        self.config.init_metadata = Some(init_metadata.as_ref().into());
        self
    }
}
