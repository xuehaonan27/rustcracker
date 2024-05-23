use serde::{Deserialize, Serialize};

use crate::{models::{balloon::Balloon, drive::Drive, logger::LogLevel, machine_configuration::MachineConfiguration, network_interface::NetworkInterface, vsock::Vsock}, RtckError, RtckErrorClass, RtckResult};

/// Firecracker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirecrackerConfig {
    /// log_path defines the file path where the Firecracker log is located.
    /// will be disabled if log_fifo is set
    pub log_path: Option<String>,

    /// log_level defines the verbosity of Firecracker logging.  Valid values are
    /// "Error", "Warning", "Info", and "Debug", and are case-sensitive.
    pub log_level: Option<LogLevel>,

    /// log_clear defines whether rustcracker should remove log files after microVM
    /// was removed. Default to false.
    pub log_clear: Option<bool>,

    /// metrics_path defines the file path where the Firecracker metrics
    /// is located.
    /// will be disabled if metrics_fifo is set
    pub metrics_path: Option<String>,

    /// metrics_clear defines whether rustcracker should remove log files after microVM
    /// was removed. Default to false.
    pub metrics_clear: Option<bool>,

    /// kernel_image_path defines the file path where the kernel image is located.
    /// The kernel image must be an uncompressed ELF image.
    pub kernel_image_path: Option<String>,

    /// initrd_path defines the file path where initrd image is located.
    /// This parameter is optional.
    pub initrd_path: Option<String>,

    /// kernel_args defines the command-line arguments that should be passed to
    /// the kernel.
    pub kernel_args: Option<String>,

    /// drives specifies BlockDevices that should be made available to the
    /// microVM.
    pub drives: Option<Vec<Drive>>,

    /// network_interfaces specifies the tap devices that should be made available
    /// to the microVM.
    pub network_interfaces: Option<Vec<NetworkInterface>>,

    /// fifo_log_writer is an io.Writer(Stdio) that is used to redirect the contents of the
    /// fifo log to the writer.
    /// pub(crate) fifo_log_writer: Option<std::process::Stdio>,
    // pub fifo_log_writer: Option<String>,

    /// vsock_devices specifies the vsock devices that should be made available to
    /// the microVM.
    pub vsock_devices: Option<Vec<Vsock>>,

    /// machine_cfg represents the firecracker microVM process configuration
    pub machine_cfg: Option<MachineConfiguration>,

    /// (Optional) vmid is a unique identifier for this VM. It's set to a
    /// random uuid if not provided by the user. It's used to set Firecracker's instance ID.
    /// If CNI configuration is provided as part of NetworkInterfaces,
    /// the vmid is used to set CNI ContainerID and create a network namespace path.
    pub vmid: Option<String>,

    /// net_ns represents the path to a network namespace handle. If present, the
    /// application will use this to join the associated network namespace
    pub net_ns: Option<String>,

    /// network_clear defines whether rustcracker should clear networking
    /// after the microVM is removed. Default to false.
    pub network_clear: Option<bool>,

    /// seccomp_level specifies whether seccomp filters should be installed and how
    /// restrictive they should be. Possible values are:
    ///
    ///	0 : (default): disabled.
    ///	1 : basic filtering. This prohibits syscalls not whitelisted by Firecracker.
    ///	2 : advanced filtering. This adds further checks on some of the
    ///			parameters of the allowed syscalls.
    pub seccomp_level: Option<usize>,

    /// mmds_address is IPv4 address used by guest applications when issuing requests to MMDS.
    /// It is possible to use a valid IPv4 link-local address (169.254.0.0/16).
    /// If not provided, the default address (169.254.169.254) will be used.
    pub mmds_address: Option<std::net::Ipv4Addr>,

    /// balloon is Balloon device that is to be put to the machine
    pub balloon: Option<Balloon>,

    /// init_metadata is initial metadata that is to be assigned to the machine
    pub init_metadata: Option<String>,
}


impl FirecrackerConfig {
    pub fn validate(&self) -> RtckResult<()> {
        todo!()
    }

    pub fn to_vec(&self) -> RtckResult<Vec<u8>> {
        Ok(serde_json::to_vec(&self)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub exec_file: Option<String>,

    // JailerBinary specifies the jailer binary to be used for setting up the
    // Firecracker VM jail.
    // If not specified it defaults to "jailer".
    pub jailer_bin: Option<String>,

    // ChrootBaseDir represents the base folder where chroot jails are built. The
    // default is /srv/jailer
    pub chroot_base_dir: Option<String>,

    //  Daemonize is set to true, call setsid() and redirect STDIN, STDOUT, and
    //  STDERR to /dev/null
    pub daemonize: Option<bool>,
}

impl JailerConfig {
    pub fn validate(&self) -> RtckResult<()> {
        todo!()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub using_jailer: bool,
    pub jailer_bin: Option<String>,
    pub jailer_config: Option<JailerConfig>,
    pub socket_path: String,
    pub frck_bin: String,
    pub frck_config: FirecrackerConfig,
    
    // Where to put firecracker exported config
    pub frck_export: Option<String>,
}

impl GlobalConfig {
    pub fn validate(&self) -> RtckResult<()> {
        if self.using_jailer {
            match &self.jailer_bin {
                Some(path) if std::fs::metadata(path).is_err() => {
                    return Err(RtckError::new(
                        RtckErrorClass::ConfigError,
                        "Jailer bin missing".to_string(),
                    ));
                }
                _ => (),
            }

            match &self.jailer_config {
                None => {
                    return Err(RtckError::new(
                        RtckErrorClass::ConfigError,
                        "Using jailer but no jailer config specified".to_string(),
                    ))
                }
                Some(config) => config.validate()?,
            }
        }

        if std::fs::metadata(&self.frck_bin).is_err() {
            return Err(RtckError::new(
                RtckErrorClass::ConfigError,
                "Firecracker bin missing".to_string(),
            ));
        }

        if std::fs::metadata(&self.socket_path).is_ok() {
            return Err(RtckError::new(
                RtckErrorClass::ConfigError,
                "Socket already exists".to_string(),
            ));
        }

        Ok(())
    }

    /// Export the firecracker config
    pub fn export_config(&self) -> RtckResult<()> {
        match &self.frck_export {
            None => Ok(()),
            Some(path) => Ok(std::fs::write(path, self.frck_config.to_vec()?)?),
        }
    }
}

// Global Config for Firecracker
