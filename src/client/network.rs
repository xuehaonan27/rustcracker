/// Bridge between firecracker and CNI plugins
use std::{
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use log::{debug, info, error};

use crate::model::{kernel_args::KernelArgs, rate_limiter::RateLimiter};

use super::handler::HandlerList;

const DEFAULT_CNI_BIN_DIR: &'static str = "/opt/cni/bin";
const DEFAULT_CNI_CONF_DIR: &'static str = "/etc/cni/conf.d";
const DEFAULT_CNI_CACHE_DIR: &'static str = "/var/lib/cni";

/// UniNetworkInterfaces is a slice of NetworkInterface objects that a VM will be
/// configured to use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniNetworkInterfaces(pub Vec<UniNetworkInterface>);

/// UniNetworkInterface represents a Firecracker microVM's network interface.
/// It can be configured either with static parameters set via StaticConfiguration
/// or via CNI as set via CNIConfiguration. It is currently an error to specify
/// both static and CNI configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniNetworkInterface {
    /// StaticConfiguration parameters that will be used to configure the VM's
    /// tap device and internal network for this network interface.
    pub static_configuration: Option<StaticNetworkConfiguration>,

    /// CNIConfiguration that will be used to generate the VM's network namespace,
    /// tap device and internal network for this network interface.
    pub cni_configuration: Option<CNIConfiguration>,

    /// AllowMMDS makes the Firecracker MMDS available on this network interface.
    pub allow_mmds: Option<bool>,

    /// InRateLimiter limits the incoming bytes.
    pub in_rate_limiter: Option<RateLimiter>,

    /// OutRateLimiter limits the outgoing bytes.
    pub out_rate_limiter: Option<RateLimiter>,
}

/// CNIConfiguration specifies the CNI parameters that will be used to generate
/// the network namespace and tap device used by a Firecracker interface.
///
/// Currently, CNIConfiguration can only be specified for VMs that have a
/// single network interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CNIConfiguration {
    /// network_name (either NetworkName or NetworkConfig are required)
    /// corresponds to the "name" parameter in the CNI spec's
    /// Network Configuration List structure. It selects the name
    /// of the network whose configuration will be used when invoking CNI.
    pub network_name: Option<String>,

    /// network_config (either NetworkName or NetworkConfig are required)
    /// replaces the NetworkName with parsed CNI network configuration
    /// skipping the requirement to store network config file in CNI
    /// configuration directory.
    pub network_config: Option<()>,

    /// if_name (optional) corresponds to the CNI_IFNAME parameter as specified
    /// in the CNI spec. It generally specifies the name of the interface to be
    /// created by a CNI plugin being invoked.
    ///
    /// Note that this does NOT necessarily correspond to the name of the
    /// tap device the Firecracker VM will use as the tap device may be
    /// created by a chained plugin that adapts the tap to a pre-existing
    /// network device (which will by the one with "IfName").
    pub if_name: Option<String>,

    /// vm_if_name (optional) sets the interface name in the VM. It is used
    /// to correctly pass IP configuration obtained from the CNI to the VM kernel.
    /// It can be left blank for VMs with single network interface.
    pub vm_if_name: Option<String>,

    /// args (optional) corresponds to the CNI_ARGS parameter as specified in
    /// the CNI spec. It allows custom args to be passed to CNI plugins during
    /// invocation.
    pub args: Option<Vec<(String, String)>>,

    /// bin_path (optional) is a list of directories in which CNI plugin binaries
    /// will be sought. If not provided, defaults to just "/opt/bin/CNI"
    pub bin_path: Option<PathBuf>,

    /// conf_dir (optional) is the directory in which CNI configuration files
    /// will be sought. If not provided, defaults to "/etc/cni/conf.d"
    pub conf_dir: Option<PathBuf>,

    /// CacheDir (optional) is the director in which CNI queries/results will be
    /// cached by the runtime. If not provided, defaults to "/var/lib/cni"
    pub cache_dir: Option<PathBuf>,

    /// containerID corresponds to the CNI_CONTAINERID parameter as
    /// specified in the CNI spec. It is private to CNIConfiguration
    /// because we expect users to provide it via the Machine's VMID parameter
    /// (or otherwise be randomly generated if the VMID was unset by the user)
    pub container_id: String,

    /// net_ns_path is private to CNIConfiguration because we expect users to
    /// either provide the netNSPath via the Jailer config or allow the
    /// netns path to be autogenerated by us.
    net_ns_path: Option<PathBuf>,

    /// force allows to overwrite default behavior of the pre existing network deletion
    /// mostly created for different types of CNI plugins which are not expecting fail on that step.
    /// In case if Force was set to `True` error will be still logged, but new new network will be created anyway.
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticNetworkConfiguration {
    /// MacAddress defines the MAC address that should be assigned to the network
    /// interface inside the microVM.
    pub mac_address: String,

    /// HostDevName is the name of the tap device the VM will use
    pub host_dev_name: Option<String>,

    /// IPConfiguration (optional) allows a static IP, gateway and up to 2 DNS nameservers
    /// to be automatically configured within the VM upon startup.
    pub ip_configuration: Option<IPConfiguration>,
}

/// IPConfiguration specifies an IP, a gateway and DNS Nameservers that should be configured
/// automatically within the VM upon boot. It currently only supports IPv4 addresses.
///
/// IPConfiguration can specify interface name, in that case config will be applied to the
/// specified interface, if IfName is left blank, config applies to VM with a single network interface.
/// The IPAddr and Gateway will be used to assign an IP a a default route for the VM's internal
/// interface.
///
/// The first 2 nameservers will be configured in the /proc/net/pnp file in a format
/// compatible with /etc/resolv.conf (any further nameservers are currently ignored). VMs that
/// wish to use the nameserver settings here will thus typically need to make /etc/resolv.conf
/// a symlink to /proc/net/pnp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPConfiguration {
    ip_addr: Ipv4Addr,
    ip_mask: Ipv4Addr,
    gateway: IpAddr,

    nameservers: Vec<String>,
    if_name: String,
}

#[derive(thiserror::Error, Debug)]
pub enum UniNetworkInterfaceError {
    #[error("Configuration error in UniNetworkInterface's setting, reason: {0}")]
    Configuration(String),
    #[error("Validation failed, reason: {0}")]
    Validation(String),
}

impl UniNetworkInterfaces {
    pub fn validate(&self, kernel_args: KernelArgs) -> Result<(), UniNetworkInterfaceError> {
        for iface in &self.0 {
            let has_cni = iface.cni_configuration.is_some();
            let has_static_interface = iface.static_configuration.is_some();
            let has_static_ip = has_static_interface
                && (iface.static_configuration.is_some()
                    && iface
                        .static_configuration
                        .as_ref()
                        .unwrap()
                        .ip_configuration
                        .is_some());

            if !has_cni && !has_static_interface {
                return Err(UniNetworkInterfaceError::Configuration(
                    "must specify at least one of CNIConfiguration or StaticConfiguration"
                        .to_string(),
                ));
            }

            // due to limitations of using "ip=" kernel boot param, currently only one network interface
            // can be provided when a static IP is going to be configured.
            if has_cni || has_static_ip {
                if self.0.len() > 1 {
                    return Err(UniNetworkInterfaceError::Configuration(format!(
                        "cannot specify CNIConfiguration or IPconfiguration when multiple network interfaces are provided: {:#?}",
                        self.0
                    )));
                }

                let arg_val = kernel_args.0.get("ip");
                if arg_val.is_some() && arg_val.as_ref().unwrap().is_some() {
                    return Err(UniNetworkInterfaceError::Configuration(format!(
                        "CNIConfiguration or IPConfiguration cannot be specified when \"ip=\" provided in kernel boot args, value found: {}",
                        arg_val.as_ref().unwrap().as_ref().unwrap()
                    )));
                }
            }

            if has_cni {
                iface.cni_configuration.as_ref().unwrap().validate()?;
            }

            if has_static_interface {
                iface.static_configuration.as_ref().unwrap().validate()?;
            }
        }
        Ok(())
    }

    // setupNetwork will invoke CNI if needed for any interfaces
    pub fn setup_network(&self, vmid: &Option<String>, net_ns_path: &Option<PathBuf>) -> Result<HandlerList, UniNetworkInterfaceError> {
        let cleanup_funcs = HandlerList::blank();
        // Get the network interface with CNI configuration or, if there is none,
	    // just return right away.
        let cni_network_interface = self.cni_interface();
        if cni_network_interface.is_none() {
            return Ok(cleanup_funcs);
        }
        let mut cni_network_interface = cni_network_interface.unwrap();
        
        cni_network_interface.cni_configuration.as_mut().unwrap().container_id = vmid.to_owned().ok_or(UniNetworkInterfaceError::Configuration("no vmid provided in Config".to_string()))?;
        // cni_network_interface.cni_configuration.as_mut().unwrap().net_ns_path = net_ns_path.to_owned();
        cni_network_interface.cni_configuration.as_mut().unwrap().net_ns_path = net_ns_path.to_owned();
        cni_network_interface.cni_configuration.as_mut().unwrap().set_defaults();

        // Make sure the netns is setup. If the path doesn't yet exist, it will be
	    // initialized with a new empty netns.
        let netns_cleanup_funs = cni_network_interface.cni_configuration.as_ref().unwrap().initialize_net_ns().map_err(|e| {
            error!(target: "UniNetworkInterfaces:setup_network", "failed to initialize netns");
            UniNetworkInterfaceError::Configuration("failed to initialize netns".to_string())
        })?;

        let cni_result = cni_network_interface.cni_configuration.as_ref().unwrap().invoke_cni().map_err(|e| {
            error!(target: "UniNetworkInterfaces::setup_network", "failure when invoking CNI");
            UniNetworkInterfaceError::Configuration("failure when invoking CNI".to_string())
        })?;

        // If static configuration is not already set for the network device, fill it out
        // by parsing the CNI result object according to the specifications detailed in the
        // vmconf package docs.

        todo!()
    }

    // return the network interface that has CNI configuration, or None if there is no such interface
    pub fn cni_interface(&self) -> Option<UniNetworkInterface> {
        // Validation that there is at most one CNI interface is done as part of the
        // NetworkConfigValidationHandler, can safely just use the first result
        // here and assume it's the only one.
        for iface in &self.0 {
            if iface.cni_configuration.is_some() {
                return Some(iface.to_owned())
            }
        }

        None
    }

    // return the network interface that has static IP configuration, or nil if there is no such interface
    pub fn static_ip_interface(&self) -> Option<UniNetworkInterface> {
        // Validation that there is at most one interface with StaticIPConfiguration
        // is done as part of the NetworkConfigValidationHandler, can safely just use
        // the first result here and assume it's the only one.
        for iface in &self.0 {
            if iface.static_configuration.is_none() {
                continue;
            } else if iface.static_configuration.as_ref().unwrap().ip_configuration.is_some() {
                return Some(iface.to_owned())
            }
        }

        None
    }
}

impl UniNetworkInterface {}

impl CNIConfiguration {
    pub(crate) fn validate(&self) -> Result<(), UniNetworkInterfaceError> {
        if self.network_name.is_none() && self.network_config.is_none() {
            return Err(UniNetworkInterfaceError::Validation(format!(
                "must specify either network_name or network_config in CNIConfiguration: {:#?}", self
            )));
        }

        if self.network_name.is_some() && self.network_config.is_some() {
            return Err(UniNetworkInterfaceError::Validation(format!(
                "must not specify both network_name and network_config in CNIConfiguration: {:#?}", self
            )));
        }

        Ok(())
    }

    pub(crate) fn set_defaults(&mut self) {
        if self.bin_path.is_none() {
            self.bin_path = Some(DEFAULT_CNI_BIN_DIR.into());
        }

        if self.conf_dir.is_none() {
            self.conf_dir = Some(DEFAULT_CNI_CONF_DIR.into());
        }

        if self.cache_dir.is_none() {
            let path: PathBuf = [DEFAULT_CNI_CACHE_DIR.to_owned(), self.container_id.to_owned().into()].iter().collect();
            self.cache_dir = Some(path);
        }
    }

    pub(crate) fn as_cni_runtime_conf(&self) -> cni_plugin::config::RuntimeConfig {
        todo!()
    }

    pub(crate) fn invoke_cni(&self) -> Result<HandlerList, UniNetworkInterfaceError> {
        todo!()
    }

    /// initializeNetNS checks to see if the netNSPath already exists, if it doesn't it will create
    /// a new one mounted at that path.
    pub(crate) fn initialize_net_ns(&self) -> Result<HandlerList, UniNetworkInterfaceError> {
        todo!()
    }
}

impl StaticNetworkConfiguration {
    pub(crate) fn validate(&self) -> Result<(), UniNetworkInterfaceError> {
        if self.host_dev_name.is_none() || self.host_dev_name.as_ref().unwrap() == &"".to_string() {
            return Err(UniNetworkInterfaceError::Validation(format!(
                "host_dev_name must be provided if StaticNetworkConfiguration is provided, {:#?}", self
            )));
        }

        if self.ip_configuration.is_some() {
            self.ip_configuration.as_ref().unwrap().validate()?;
        }

        Ok(())
    }
}

impl IPConfiguration {
    pub(crate) fn validate(&self) -> Result<(), UniNetworkInterfaceError> {
        // make sure only ipv4 is being provided (for now).
        if !self.gateway.is_ipv4() {
            return Err(UniNetworkInterfaceError::Validation(format!(
                "invalid ip, only ipv4 address are supported: {}",
                self.gateway
            )));
        }

        if self.nameservers.len() > 2 {
            return Err(UniNetworkInterfaceError::Validation(format!(
                "cannot specify more than 2 nameservers: {:#?}",
                self.nameservers
            )));
        }

        Ok(())
    }

    ///IPBootParam provides a string that can be used as the argument to "ip=" in a Linux kernel boot
    /// parameters in order to boot a machine with network settings matching those in a StaticNetworkConf
    /// object.
    ///
    /// See "ip=" section of kernel docs here for more details:
    /// https://www.kernel.org/doc/Documentation/filesystems/nfs/nfsroot.txt
    ///
    /// Due to the limitation of "ip=", not all configuration specified in StaticNetworkConf can be
    /// applied automatically. In particular:
    /// * The MacAddr and MTU cannot be applied
    /// * The only routes created will match what's specified in VMIPConfig; VMRoutes will be ignored.
    /// * Only up to two namesevers can be supplied. If VMNameservers is has more than 2 entries, only
    ///   the first two in the slice will be applied in the VM.
    /// * VMDomain, VMSearchDomains and VMResolverOptions will be ignored
    /// * Nameserver settings are also only set in /proc/net/pnp. Most applications will thus require
    ///   /etc/resolv.conf to be a symlink to /proc/net/pnp in order to resolve names as expected.
    pub(crate) fn ip_boot_param(&self) -> String {
        // See "ip=" section of kernel linked above for details on each field listed below.

        // client-ip is really just the ip that will be assigned to the primary interface
        let client_ip = self.ip_addr.to_string();

        // don't set nfs server IP
        const SERVER_IP: &'static str = "";

        // default gateway for the network; used to generate a corresponding route table entry
        let default_gateway = self.gateway.to_string();

        // subnet mask used to generate a corresponding route table entry for the primary interface
        // (must be provided in dotted decimal notation)

        let subnet_mask = self.ip_mask.to_string();

        // the "hostname" field actually just configures a hostname value for DHCP requests, thus no need to set it
        const DHCP_HOST_NAME: &'static str = "";

        // if blank, use the only network device present in the VM
        let device = self.if_name.to_owned();

        // Don't do any autoconfiguration (i.e. DHCP, BOOTP, RARP)
        const AUTO_CONFIGURATION: &'static str = "off";

        // up to two nameservers (if any were provided)
        let mut nameservers = [""; 2];
        for (i, ns) in self.nameservers.iter().enumerate() {
            if i == 2 {
                break;
            }
            nameservers[i] = ns.as_str();
        }

        // TODO(sipsma) should we support configuring an NTP server?
        const NTP_SERVER: &'static str = "";

        [
            client_ip,
            SERVER_IP.to_string(),
            default_gateway,
            subnet_mask,
            DHCP_HOST_NAME.to_string(),
            device,
            AUTO_CONFIGURATION.to_string(),
            nameservers[0].to_string(),
            nameservers[1].to_string(),
            NTP_SERVER.to_string(),
        ]
        .join(":")
    }
}
