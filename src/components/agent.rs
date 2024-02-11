use std::path::PathBuf;

use hyper::{Body, Client, Method, Request};
use hyperlocal::{UnixClientExt, UnixConnector};
use log::{debug, error, trace};
use tokio::time::timeout;

use crate::{
    model::{
        balloon::Balloon,
        balloon_stats::BalloonStatistics,
        balloon_stats_update::BalloonStatsUpdate,
        balloon_update::BalloonUpdate,
        boot_source::BootSource,
        cpu_template::CPUConfig,
        drive::Drive,
        entropy_device::EntropyDevice,
        firecracker_version::FirecrackerVersion,
        full_vm_configuration::FullVmConfiguration,
        instance_action_info::InstanceActionInfo,
        instance_info::InstanceInfo,
        logger::Logger,
        machine_configuration::MachineConfiguration,
        metrics::Metrics,
        mmds_config::{MmdsConfig, MmdsContentsObject},
        network_interface::NetworkInterface,
        partial_drive::PartialDrive,
        partial_network_interface::PartialNetworkInterface,
        snapshot_create_params::SnapshotCreateParams,
        snapshot_load_params::SnapshotLoadParams,
        vm::Vm,
        vsock::Vsock,
    },
    utils::{
        Json, DEFAULT_FIRECRACKER_INIT_TIMEOUT_SECONDS, DEFAULT_FIRECRACKER_REQUEST_TIMEOUT_SECONDS,
    },
};

#[derive(thiserror::Error, Debug)]
pub enum AgentError {
    #[error("Could not initate worksapce for machine, reason: {0}")]
    WorkspaceCreation(String),
    #[error("Could not delete worksapce for machine, reason: {0}")]
    WorkspaceDeletion(String),
    #[error("Could not execute command, reason: {0}")]
    CommandExecution(String),
    #[error("Failed to manage socket, reason: {0}")]
    Socket(String),
    #[error("Could not send request on uri {0}, reason: {1}")]
    Request(hyper::Uri, String),
    #[error("Could not serialize request or deserialize response, reason: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Socket didn't start on time")]
    Unhealthy,
}

pub struct Agent {
    pub(super) socket_path: PathBuf,
    client: Client<UnixConnector>,
    pub(super) firecracker_request_timeout: u64,
    pub(super) firecracker_init_timeout: u64,
}

impl Agent {
    pub(super) fn blank() -> Self {
        Agent {
            socket_path: "".into(),
            client: Client::unix(),
            firecracker_request_timeout: DEFAULT_FIRECRACKER_REQUEST_TIMEOUT_SECONDS as u64,
            firecracker_init_timeout: DEFAULT_FIRECRACKER_INIT_TIMEOUT_SECONDS as u64,
        }
    }

    pub fn new(socket_path: &PathBuf, request_timeout: u64, init_timeout: u64) -> Self {
        Agent {
            socket_path: socket_path.to_path_buf(),
            client: Client::unix(),
            firecracker_request_timeout: request_timeout,
            firecracker_init_timeout: init_timeout,
        }
    }

    async fn send_request(
        &self,
        url: hyper::Uri,
        method: Method,
        body: String,
    ) -> Result<String, AgentError> {
        debug!("Send request to socket: {}", url);
        trace!("Sent body to socket [{}]: {}", url, body);
        let request = Request::builder()
            .method(method)
            .uri(url.clone())
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(Body::from(body))
            .map_err(|e| AgentError::Request(url.clone(), e.to_string()))?;

        let response = timeout(
            tokio::time::Duration::from_secs(self.firecracker_request_timeout),
            self.client.request(request),
        )
        .await
        .map_err(|e| {
            error!(
                target: "Agent::send_request",
                "timeout after {} seconds: {}",
                self.firecracker_request_timeout,
                e
            );
            AgentError::Request(
                url.clone(),
                format!(
                    "requesting: {} timeout after {} seconds",
                    url, self.firecracker_request_timeout
                ),
            )
        })?
        .map_err(|e| AgentError::Request(url.clone(), e.to_string()))?;
        // let response = self
        //     .client
        //     .request(request)
        //     .await
        //     .map_err(|e| AgentError::Request(url.clone(), e.to_string()))?;

        trace!("Response status: {:#?}", response.status());

        let status = response.status();
        if !status.is_success() {
            error!("Request to socket failed [{}]: {:#?}", url, status);
            // body stream to string
            let body = hyper::body::to_bytes(response.into_body())
                .await
                .map_err(|e| AgentError::Request(url.clone(), e.to_string()))?;
            error!(
                "Request [{}] body: {}",
                url,
                String::from_utf8(body.to_vec()).unwrap()
            );
            return Err(AgentError::CommandExecution(format!(
                "Failed to send request to {}, status: {}",
                url, status
            )));
        } else {
            let body = hyper::body::to_bytes(response.into_body())
                .await
                .map_err(|e| AgentError::Request(url.clone(), e.to_string()))?;
            let string = String::from_utf8(body.to_vec()).unwrap();
            Ok(string)
        }
    }

    // PUT /snapshot/create
    pub async fn create_snapshot(
        &self,
        snapshot_create_params: &SnapshotCreateParams,
    ) -> Result<(), AgentError> {
        debug!("create_snapshot: {:#?}", snapshot_create_params);
        let json = snapshot_create_params
            .to_json()
            .map_err(AgentError::Serde)?;

        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/snapshot/create").into();
        self.send_request(url, Method::PUT, json).await?;
        Ok(())
    }

    // PUT /actions
    pub async fn create_sync_action(&self, action: &InstanceActionInfo) -> Result<(), AgentError> {
        debug!("create_sync_action: {:#?}", action);
        let json = action.to_json().map_err(AgentError::Serde)?;

        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/actions").into();
        self.send_request(url, Method::PUT, json).await?;
        Ok(())
    }

    // GET /balloon
    pub async fn describe_balloon_config(&self) -> Result<Balloon, AgentError> {
        debug!("describe_balloon_config");
        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/balloon").into();
        let string = self.send_request(url, Method::GET, String::new()).await?;
        let balloon = Balloon::from_json(&string).map_err(AgentError::Serde)?;
        Ok(balloon)
    }

    // GET /balloon/statistics
    pub async fn describe_balloon_stats(&self) -> Result<BalloonStatistics, AgentError> {
        debug!("describe_balloon_stats");
        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/balloon/statistics").into();
        let string = self.send_request(url, Method::GET, String::new()).await?;
        let balloon_stats = BalloonStatistics::from_json(&string).map_err(AgentError::Serde)?;
        Ok(balloon_stats)
    }

    // GET /
    pub async fn describe_instance(&self) -> Result<InstanceInfo, AgentError> {
        debug!("describe_instance");
        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/").into();
        let string = self.send_request(url, Method::GET, String::new()).await?;
        let instance_info = InstanceInfo::from_json(&string).map_err(AgentError::Serde)?;
        Ok(instance_info)
    }

    // GET /vm/config
    pub async fn get_export_vm_config(&self) -> Result<FullVmConfiguration, AgentError> {
        debug!("get_export_vm_config");
        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/vm/config").into();
        let string = self.send_request(url, Method::GET, String::new()).await?;
        let vm_config = FullVmConfiguration::from_json(&string).map_err(AgentError::Serde)?;
        Ok(vm_config)
    }

    // GET /version
    pub async fn get_firecracker_version(&self) -> Result<FirecrackerVersion, AgentError> {
        debug!("get_firecracker_version");
        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/version").into();
        let string = self.send_request(url, Method::GET, String::new()).await?;
        let version = FirecrackerVersion::from_json(&string).map_err(AgentError::Serde)?;
        Ok(version)
    }

    // GET /machine-config
    pub(super) async fn get_machine_configuration(&self) -> Result<MachineConfiguration, AgentError> {
        debug!("get_machine_configuration");
        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/machine-config").into();
        let string = self.send_request(url, Method::GET, String::new()).await?;
        let machine_config = MachineConfiguration::from_json(&string).map_err(AgentError::Serde)?;
        Ok(machine_config)
    }

    // GET /mmds
    pub async fn get_mmds(&self) -> Result<String, AgentError> {
        debug!("get_mmds");
        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/mmds").into();
        let string = self.send_request(url, Method::GET, String::new()).await?;
        Ok(string)
    }

    // PATCH /balloon/statistics
    pub async fn patch_balloon_stats_interval(
        &self,
        balloon_stats_update: &BalloonStatsUpdate,
    ) -> Result<(), AgentError> {
        debug!("patch_balloon_stats_interval: {:#?}", balloon_stats_update);
        let json = balloon_stats_update.to_json().map_err(AgentError::Serde)?;

        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/balloon/statistics").into();
        self.send_request(url, Method::PATCH, json).await?;
        Ok(())
    }

    // PATCH /balloon
    pub async fn patch_balloon(&self, balloon_update: &BalloonUpdate) -> Result<(), AgentError> {
        debug!("patch_balloon: {:#?}", balloon_update);
        let json = balloon_update.to_json().map_err(AgentError::Serde)?;

        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/balloon").into();
        self.send_request(url, Method::PATCH, json).await?;
        Ok(())
    }

    // PATCH /drives/{drive_id}
    pub async fn patch_guest_drive_by_id(
        &self,
        partial_drive: &PartialDrive,
    ) -> Result<(), AgentError> {
        debug!("patch_guest_drive_by_id: {:#?}", partial_drive);
        let drive_id = partial_drive.get_drive_id();
        let json = partial_drive.to_json().map_err(AgentError::Serde)?;

        let url: hyper::Uri =
            hyperlocal::Uri::new(&self.socket_path, format!("/drives/{drive_id}").as_str()).into();
        self.send_request(url, Method::PATCH, json).await?;
        Ok(())
    }

    // PATCH /network-interfaces/{iface_id}
    pub async fn patch_guest_network_interface_by_id(
        &self,
        partial_network_interface: &PartialNetworkInterface,
    ) -> Result<(), AgentError> {
        debug!(
            "patch_guest_network_interface_by_id: {:#?}",
            partial_network_interface
        );
        let iface_id = partial_network_interface.get_iface_id();
        let json = partial_network_interface
            .to_json()
            .map_err(AgentError::Serde)?;

        let url: hyper::Uri = hyperlocal::Uri::new(
            &self.socket_path,
            format!("/network-interfaces/{iface_id}").as_str(),
        )
        .into();
        self.send_request(url, Method::PATCH, json).await?;
        Ok(())
    }

    // PATCH /machine-config
    pub async fn patch_machine_configuration(
        &self,
        machine_config: &MachineConfiguration,
    ) -> Result<(), AgentError> {
        debug!("patch_machine_configuration: {:#?}", machine_config);
        let json = machine_config.to_json().map_err(AgentError::Serde)?;

        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/machine-config").into();
        self.send_request(url, Method::PATCH, json).await?;
        Ok(())
    }

    // PATCH /mmds
    pub async fn patch_mmds(
        &self,
        mmds_contents_object: &MmdsContentsObject,
    ) -> Result<(), AgentError> {
        debug!("patch_mmds: {:#?}", mmds_contents_object);
        let json = mmds_contents_object.to_json().map_err(AgentError::Serde)?;

        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/mmds").into();
        self.send_request(url, Method::PATCH, json).await?;
        Ok(())
    }

    // PATCH /vm
    pub async fn patch_vm(&self, vm: &Vm) -> Result<(), AgentError> {
        debug!("patch_vm: {:#?}", vm);
        let json = vm.to_json().map_err(AgentError::Serde)?;

        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/vm").into();
        self.send_request(url, Method::PATCH, json).await?;
        Ok(())
    }

    // PUT /snapshot/load
    pub async fn load_snapshot(
        &self,
        snapshot_load_params: &SnapshotLoadParams,
    ) -> Result<(), AgentError> {
        debug!("load_snapshot: {:#?}", snapshot_load_params);
        let json = snapshot_load_params.to_json().map_err(AgentError::Serde)?;

        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/snapshot/load").into();
        self.send_request(url, Method::PUT, json).await?;
        Ok(())
    }

    // PUT /balloon
    pub async fn put_balloon(&self, balloon: &Balloon) -> Result<(), AgentError> {
        debug!("put_balloon: {:#?}", balloon);
        let json = balloon.to_json().map_err(AgentError::Serde)?;

        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/balloon").into();
        self.send_request(url, Method::PUT, json).await?;
        Ok(())
    }

    // PUT /cpu-config
    pub async fn put_cpu_configuration(&self, cpu_config: &CPUConfig) -> Result<(), AgentError> {
        debug!("put_cpu_configuration: {:#?}", cpu_config);
        let json = cpu_config.to_json().map_err(AgentError::Serde)?;

        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/cpu-config").into();
        self.send_request(url, Method::PUT, json).await?;
        Ok(())
    }

    // PUT /entropy
    pub async fn put_entropy_device(
        &self,
        entropy_device: &EntropyDevice,
    ) -> Result<(), AgentError> {
        debug!("put_entropy_device: {:#?}", entropy_device);
        let json = entropy_device.to_json().map_err(AgentError::Serde)?;

        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/entropy").into();
        self.send_request(url, Method::PUT, json).await?;
        Ok(())
    }

    // PUT /boot-source
    pub async fn put_guest_boot_source(&self, boot_source: &BootSource) -> Result<(), AgentError> {
        debug!("put_guest_boot_source: {:#?}", boot_source);
        let json = boot_source.to_json().map_err(AgentError::Serde)?;

        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/boot-source").into();
        self.send_request(url, Method::PUT, json).await?;
        Ok(())
    }

    // PUT /drives/{drive_id}
    pub async fn put_guest_drive_by_id(&self, drive: &Drive) -> Result<(), AgentError> {
        debug!("put_guest_drive_by_id: {:#?}", drive);
        let drive_id = drive.get_drive_id();
        let json = drive.to_json().map_err(AgentError::Serde)?;

        let url: hyper::Uri =
            hyperlocal::Uri::new(&self.socket_path, format!("/drives/{drive_id}").as_str()).into();
        self.send_request(url, Method::PUT, json).await?;
        Ok(())
    }

    // PUT /network-interfaces/{iface_id}
    pub async fn put_guest_network_interface_by_id(
        &self,
        network_interface: &NetworkInterface,
    ) -> Result<(), AgentError> {
        debug!(
            "put_guest_network_interface_by_id: {:#?}",
            network_interface
        );
        let iface_id = network_interface.get_iface_id();
        let json = network_interface.to_json().map_err(AgentError::Serde)?;

        let url: hyper::Uri = hyperlocal::Uri::new(
            &self.socket_path,
            format!("/network-interfaces/{iface_id}").as_str(),
        )
        .into();
        self.send_request(url, Method::PUT, json).await?;
        Ok(())
    }

    // PUT /vsock
    pub async fn put_guest_vsock(&self, vsock: &Vsock) -> Result<(), AgentError> {
        debug!("put_guest_vsock: {:#?}", vsock);
        let json = vsock.to_json().map_err(AgentError::Serde)?;

        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/vsock").into();
        self.send_request(url, Method::PUT, json).await?;
        Ok(())
    }

    // PUT /logger
    pub async fn put_logger(&self, logger: &Logger) -> Result<(), AgentError> {
        debug!("put_logger: {:#?}", logger);
        let json = logger.to_json().map_err(AgentError::Serde)?;

        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/logger").into();
        self.send_request(url, Method::PUT, json).await?;
        Ok(())
    }

    // PUT /machine-config
    pub async fn put_machine_configuration(
        &self,
        machine_config: &MachineConfiguration,
    ) -> Result<(), AgentError> {
        debug!("put_machine_configuration: {:#?}", machine_config);
        let json = machine_config.to_json().map_err(AgentError::Serde)?;

        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/machine-config").into();
        self.send_request(url, Method::PUT, json).await?;
        Ok(())
    }

    // PUT /metrics
    pub async fn put_metrics(&self, metrics: &Metrics) -> Result<(), AgentError> {
        debug!("put_metrics: {:#?}", metrics);
        let json = metrics.to_json().map_err(AgentError::Serde)?;

        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/metrics").into();
        self.send_request(url, Method::PUT, json).await?;
        Ok(())
    }

    // PUT /mmds/config
    pub async fn put_mmds_config(&self, mmds_config: &MmdsConfig) -> Result<(), AgentError> {
        debug!("put_mmds_config: {:#?}", mmds_config);
        let json = mmds_config.to_json().map_err(AgentError::Serde)?;

        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/mmds/config").into();
        self.send_request(url, Method::PUT, json).await?;
        Ok(())
    }

    // PUT /mmds
    pub async fn put_mmds(
        &self,
        mmds_contents_object: &MmdsContentsObject,
    ) -> Result<(), AgentError> {
        debug!("put_mmds: {:#?}", mmds_contents_object);
        let url: hyper::Uri = hyperlocal::Uri::new(&self.socket_path, "/mmds").into();

        self.send_request(url, Method::PUT, mmds_contents_object.to_string())
            .await?;
        Ok(())
    }
}
