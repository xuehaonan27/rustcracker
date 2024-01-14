use std::path::PathBuf;

use async_channel::Receiver;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    sync::oneshot,
};

use crate::{
    client::http_client::res_into_parts,
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
    utils::Json,
};

use super::http_client::http_request;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

const MAX_PENDING_REQUEST_NUM: usize = 64;
const MAX_BUFFER_LENGTH: usize = 1024;
pub struct SocketConnectionPool {
    /* 最多连接个数 */
    max_conn_num: usize,

    /* 每一个连接管道中最多等待的请求数量 */
    max_pending_request_num: usize,

    /* 连接池runtime使用的线程数量 */
    worker_threads: usize,

    /* 目标套接字地址 */
    socket_path: PathBuf,

    /* 连接池入口 */
    pub actor_handle: ActorHandle,

    /* 运行时 */
    #[allow(unused)]
    runtime: tokio::runtime::Runtime,
}

impl SocketConnectionPool {
    pub fn new(
        socket_path: PathBuf,
        max_conn_num: usize,
        max_pending_request_num: usize,
        worker_threads: usize,
    ) -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(worker_threads)
            .build()
            .unwrap();

        let (actor_handle, receiver) =
            ActorHandle::pair(&socket_path, &rt).expect("Fail to create socket connection pool");

        for i in 1..max_conn_num {
            let actor =
                Actor::build(i, &socket_path, receiver.clone()).expect("Fail to copy the actor");
            rt.spawn(run_my_actor(actor));
        }
        Self {
            max_conn_num,
            max_pending_request_num,
            worker_threads,
            socket_path,
            actor_handle,
            runtime: rt,
        }
    }

    /* Getter */
    pub fn max_conn_num(&self) -> usize {
        self.max_conn_num
    }

    /* Getter */
    pub fn max_pending_request_num(&self) -> usize {
        self.max_pending_request_num
    }

    /* Getter */
    pub fn worker_threads(&self) -> usize {
        self.worker_threads
    }

    /* Getter */
    pub fn socket_path(&self) -> PathBuf {
        self.socket_path.clone()
    }

    /* Getter */
    pub fn actor_handle(&self) -> ActorHandle {
        self.actor_handle.clone()
    }
}

/* 对于一个Unix Domain Socket, 可以建立一个连接池, 内部有多个Actor以及可以复制的连接池入口ActorHandle, 每一个Actor管理一个对该UDS的连接UnixStream */
/* 请求被连接池委派给某一个Actor */
pub struct Actor {
    // receiver: mpsc::Receiver<ActorMessage>,
    receiver: async_channel::Receiver<ActorMessage>,
    id: usize,
    socket_path: PathBuf,
    // agent: Option<SendRequest<Full<Bytes>>>,
    conn: UnixStream,
}

pub(crate) enum ActorMessage {
    InspectId {
        respond_to: oneshot::Sender<usize>,
    },
    InspectSocketPath {
        respond_to: oneshot::Sender<PathBuf>,
    },

    // PUT /snapshot/create
    CreateSnapshot {
        respond_to: oneshot::Sender<(String, String)>,
        snapshot_create_params: SnapshotCreateParams,
    },

    // PUT /actions
    CreateSyncAction {
        respond_to: oneshot::Sender<(String, String)>,
        action: InstanceActionInfo,
    },

    // GET /balloon
    DescribeBalloonConfig {
        respond_to: oneshot::Sender<Balloon>,
    },

    // GET /balloon/statistics
    DescribeBalloonStats {
        respond_to: oneshot::Sender<BalloonStatistics>,
    },

    // GET /
    DescribeInstance {
        respond_to: oneshot::Sender<InstanceInfo>,
    },

    // GET /vm/config
    GetExportVmConfig {
        respond_to: oneshot::Sender<FullVmConfiguration>,
    },

    // GET /version
    GetFirecrackerVersion {
        respond_to: oneshot::Sender<FirecrackerVersion>,
    },

    // GET /machine-config
    GetMachineConfiguration {
        respond_to: oneshot::Sender<MachineConfiguration>,
    },

    // GET /mmds
    GetMmds {
        respond_to: oneshot::Sender<String>,
    },

    // PUT /snapshot/load
    LoadSnapshot {
        respond_to: oneshot::Sender<(String, String)>,
        snapshot_load_params: SnapshotLoadParams,
    },

    // PATCH /balloon/statistics
    PatchBalloonStatsInterval {
        respond_to: oneshot::Sender<(String, String)>,
        balloon_stats_update: BalloonStatsUpdate,
    },

    // PATCH /balloon
    PatchBalloon {
        respond_to: oneshot::Sender<(String, String)>,
        balloon_update: BalloonUpdate,
    },

    // PATCH /drives/{drive_id}
    PatchGuestDriveByID {
        respond_to: oneshot::Sender<(String, String)>,
        partial_drive: PartialDrive,
    },

    // PATCH /network-interfaces/{iface_id}
    PatchGuestNetworkInterfaceByID {
        respond_to: oneshot::Sender<(String, String)>,
        partial_network_interface: PartialNetworkInterface,
    },

    // PATCH /machine-config
    PatchMachineConfiguration {
        respond_to: oneshot::Sender<(String, String)>,
        machine_config: MachineConfiguration,
    },

    // PATCH /mmds
    PatchMmds {
        respond_to: oneshot::Sender<(String, String)>,
        mmds_contents_object: MmdsContentsObject,
    },

    // PATCH /vm
    PatchVm {
        respond_to: oneshot::Sender<(String, String)>,
        vm: Vm,
    },

    // PUT /balloon
    PutBalloon {
        respond_to: oneshot::Sender<(String, String)>,
        balloon: Balloon,
    },

    // PUT /cpu-config
    PutCpuConfiguration {
        respond_to: oneshot::Sender<(String, String)>,
        cpu_config: CPUConfig,
    },

    // PUT /entropy
    PutEntropyDevice {
        respond_to: oneshot::Sender<(String, String)>,
        entropy_device: EntropyDevice,
    },

    // PUT /boot-source
    PutGuestBootSource {
        respond_to: oneshot::Sender<(String, String)>,
        boot_source: BootSource,
    },

    // PUT /drives/{drive_id}
    PutGuestDriveByID {
        respond_to: oneshot::Sender<(String, String)>,
        drive: Drive,
    },

    // PUT /network-interfaces/{iface_id}
    PutGuestNetworkInterfaceByID {
        respond_to: oneshot::Sender<(String, String)>,
        network_interface: NetworkInterface,
    },

    // PUT /vsock
    PutGuestVsock {
        respond_to: oneshot::Sender<(String, String)>,
        vsock: Vsock,
    },

    // PUT /logger
    PutLogger {
        respond_to: oneshot::Sender<(String, String)>,
        logger: Logger,
    },

    // PUT /machine-config
    PutMachineConfiguration {
        respond_to: oneshot::Sender<(String, String)>,
        machine_config: MachineConfiguration,
    },

    // PUT /metrics
    PutMetrics {
        respond_to: oneshot::Sender<(String, String)>,
        metrics: Metrics,
    },

    // PUT /mmds/config
    PutMmdsConfig {
        respond_to: oneshot::Sender<(String, String)>,
        mmds_config: MmdsConfig,
    },

    // PUT /mmds
    PutMmds {
        respond_to: oneshot::Sender<(String, String)>,
        mmds_contents_object: MmdsContentsObject,
    },
}

impl Actor {
    pub fn id(&self) -> usize {
        self.id
    }

    fn build(
        id: usize,
        socket_path: &PathBuf,
        receiver: async_channel::Receiver<ActorMessage>,
    ) -> Result<Self> {
        // let rt = tokio::runtime::Builder::new_current_thread()
        //     .enable_all()
        //     .build()?;

        // let conn = rt.block_on(UnixStream::connect(socket_path))?;
        // let conn = rt.block_on(UnixStream::connect(socket_path))?;
        // drop(rt); /* 显式释放rt, 尽快释放资源 */
        let std_stream = std::os::unix::net::UnixStream::connect(&socket_path)?;
        std_stream.set_nonblocking(true)?;
        let conn = UnixStream::from_std(std_stream)?;

        Ok(Actor {
            receiver,
            id,
            socket_path: socket_path.clone(),
            conn,
        })
    }

    async fn handle_message(&mut self, msg: ActorMessage) -> Result<()> {
        match msg {
            ActorMessage::InspectId { respond_to } => {
                let _ = respond_to.send(self.id);
                Ok(())
            }
            ActorMessage::InspectSocketPath { respond_to } => {
                let _ = respond_to.send(self.socket_path.clone());
                Ok(())
            }

            // PUT /snapshot/create
            ActorMessage::CreateSnapshot {
                respond_to,
                snapshot_create_params,
            } => {
                let request = http_request(
                    "PUT",
                    "/snapshot/create",
                    Some(snapshot_create_params.to_json()?),
                );
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // PUT /actions
            ActorMessage::CreateSyncAction { respond_to, action } => {
                let request = http_request("PUT", "/actions", Some(action.to_json()?));
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // GET /balloon
            ActorMessage::DescribeBalloonConfig { respond_to } => {
                let request = http_request("GET", "/balloon", None);
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (_status_code, _status_text, body) = res_into_parts(response);
                let _ = respond_to.send(Balloon::from_json(&body)?);
                Ok(())
            }

            // GET /balloon/statistics
            ActorMessage::DescribeBalloonStats { respond_to } => {
                let request = http_request("GET", "/balloon/statistics", None);
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (_status_code, _status_text, body) = res_into_parts(response);
                let _ = respond_to.send(BalloonStatistics::from_json(&body)?);
                Ok(())
            }

            // GET /
            ActorMessage::DescribeInstance { respond_to } => {
                let request = http_request("GET", "/", None);
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (_status_code, _status_text, body) = res_into_parts(response);
                let _ = respond_to.send(InstanceInfo::from_json(&body)?);
                Ok(())
            }

            // GET /vm/config
            ActorMessage::GetExportVmConfig { respond_to } => {
                let request = http_request("GET", "/vm/config", None);
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (_status_code, _status_text, body) = res_into_parts(response);
                let _ = respond_to.send(FullVmConfiguration::from_json(&body)?);
                Ok(())
            }

            // GET /version
            ActorMessage::GetFirecrackerVersion { respond_to } => {
                let request = http_request("GET", "/version", None);
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (_status_code, _status_text, body) = res_into_parts(response);
                let _ = respond_to.send(FirecrackerVersion::from_json(&body)?);
                Ok(())
            }

            // GET /machine-config
            ActorMessage::GetMachineConfiguration { respond_to } => {
                let request = http_request("GET", "/machine-config", None);
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (_status_code, _status_text, body) = res_into_parts(response);
                let _ = respond_to.send(MachineConfiguration::from_json(&body)?);
                Ok(())
            }

            // GET /mmds
            ActorMessage::GetMmds { respond_to } => {
                let request = http_request("GET", "/mmds", None);
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (_status_code, _status_text, body) = res_into_parts(response);
                let _ = respond_to.send(body);
                Ok(())
            }

            // PUT /snapshot/load
            ActorMessage::LoadSnapshot {
                respond_to,
                snapshot_load_params,
            } => {
                let request = http_request(
                    "PUT",
                    "/snapshot/load",
                    Some(snapshot_load_params.to_json()?),
                );
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // PATCH /balloon/statistics
            ActorMessage::PatchBalloonStatsInterval {
                respond_to,
                balloon_stats_update,
            } => {
                let request = http_request(
                    "PATCH",
                    "/balloon/statistics",
                    Some(balloon_stats_update.to_json()?),
                );
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // PATCH /balloon
            ActorMessage::PatchBalloon {
                respond_to,
                balloon_update,
            } => {
                let request = http_request("PATCH", "/balloon", Some(balloon_update.to_json()?));
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // PATCH /drives/{drive_id}
            ActorMessage::PatchGuestDriveByID {
                respond_to,
                partial_drive,
            } => {
                let drive_id = partial_drive.get_drive_id();
                let request = http_request(
                    "PATCH",
                    format!("/drives/{drive_id}").as_str(),
                    Some(partial_drive.to_json()?),
                );
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // PATCH /network-interfaces/{iface_id}
            ActorMessage::PatchGuestNetworkInterfaceByID {
                respond_to,
                partial_network_interface,
            } => {
                let iface_id = partial_network_interface.get_iface_id();
                let request = http_request(
                    "PATCH",
                    format!("/network-interfaces/{iface_id}").as_str(),
                    Some(partial_network_interface.to_json()?),
                );
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // PATCH /machine-config
            ActorMessage::PatchMachineConfiguration {
                respond_to,
                machine_config,
            } => {
                let request =
                    http_request("PATCH", "/machine-config", Some(machine_config.to_json()?));
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // PATCH /mmds
            ActorMessage::PatchMmds {
                respond_to,
                mmds_contents_object,
            } => {
                let request = http_request("PATCH", "/mmds", Some(mmds_contents_object.to_json()?));
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // PATCH /vm
            ActorMessage::PatchVm { respond_to, vm } => {
                let request = http_request("PATCH", "/vm", Some(vm.to_json()?));
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // PUT /balloon
            ActorMessage::PutBalloon {
                respond_to,
                balloon,
            } => {
                let request = http_request("PUT", "/balloon", Some(balloon.to_json()?));
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // PUT /cpu-config
            ActorMessage::PutCpuConfiguration {
                respond_to,
                cpu_config,
            } => {
                let request = http_request("PUT", "/cpu-config", Some(cpu_config.to_json()?));
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // PUT /entropy
            ActorMessage::PutEntropyDevice {
                respond_to,
                entropy_device,
            } => {
                let request = http_request("PUT", "/entropy", Some(entropy_device.to_json()?));
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // PUT /boot-source
            ActorMessage::PutGuestBootSource {
                respond_to,
                boot_source,
            } => {
                let request = http_request("PUT", "/boot-source", Some(boot_source.to_json()?));
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // PUT /drives/{drive_id}
            ActorMessage::PutGuestDriveByID { respond_to, drive } => {
                let drive_id = drive.get_drive_id();
                let request = http_request(
                    "PUT",
                    format!("/drives/{drive_id}").as_str(),
                    Some(drive.to_json()?),
                );
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // PUT /network-interfaces/{iface_id}
            ActorMessage::PutGuestNetworkInterfaceByID {
                respond_to,
                network_interface,
            } => {
                let iface_id = network_interface.get_iface_id();
                let request = http_request(
                    "PUT",
                    format!("/network-interfaces/{iface_id}").as_str(),
                    Some(network_interface.to_json()?),
                );
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // PUT /vsock
            ActorMessage::PutGuestVsock { respond_to, vsock } => {
                let request = http_request("PUT", "/vsock", Some(vsock.to_json()?));
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // PUT /logger
            ActorMessage::PutLogger { respond_to, logger } => {
                let request = http_request("PUT", "/logger", Some(logger.to_json()?));
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // PUT /machine-config
            ActorMessage::PutMachineConfiguration {
                respond_to,
                machine_config,
            } => {
                let request =
                    http_request("PUT", "/machine-config", Some(machine_config.to_json()?));
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // PUT /metrics
            ActorMessage::PutMetrics {
                respond_to,
                metrics,
            } => {
                let request = http_request("PUT", "/metrics", Some(metrics.to_json()?));
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // PUT /mmds/config
            ActorMessage::PutMmdsConfig {
                respond_to,
                mmds_config,
            } => {
                let request = http_request("PUT", "/mmds/config", Some(mmds_config.to_json()?));
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }

            // PUT /mmds
            ActorMessage::PutMmds {
                respond_to,
                mmds_contents_object,
            } => {
                let request = http_request("PUT", "/mmds", Some(mmds_contents_object));
                self.conn.write_all(request.as_bytes()).await?;
                let mut buf: [u8; MAX_BUFFER_LENGTH] = [0; MAX_BUFFER_LENGTH];
                let _ = self.conn.read(&mut buf).await?;
                let response = String::from_utf8(buf.to_vec())?;
                let (status_code, status_text, _body) = res_into_parts(response);
                let _ = respond_to.send((status_code, status_text));
                Ok(())
            }
        }
    }
}

async fn run_my_actor(mut actor: Actor) {
    let id = actor.id();
    while let Ok(msg) = actor.receiver.recv().await {
        println!("Actor {} got a message", id);
        actor.handle_message(msg).await.unwrap(); // more graceful error handling needed
    }
}

#[derive(Clone)]
pub struct ActorHandle {
    // sender: mpsc::Sender<ActorMessage>,
    sender: async_channel::Sender<ActorMessage>,
}

impl ActorHandle {
    pub(crate) fn pair(
        socket_path: &PathBuf,
        rt: &tokio::runtime::Runtime,
    ) -> Result<(Self, Receiver<ActorMessage>)> {
        let (sender, receiver) = async_channel::bounded(MAX_PENDING_REQUEST_NUM);
        let actor = Actor::build(0, socket_path, receiver.clone())?;
        rt.spawn(run_my_actor(actor));
        Ok((Self { sender }, receiver))
    }

    pub async fn inspect_id(&self) -> usize {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::InspectId { respond_to: send };

        let _ = self.sender.send(msg).await; /* 这里可以改成别的channel进行广播 */
        recv.await.expect("Actor task has been killed")
    }

    pub async fn inspect_socket_path(&self) -> PathBuf {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::InspectSocketPath { respond_to: send };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn create_snap_shot(
        &self,
        snapshot_create_params: SnapshotCreateParams,
    ) -> (String, String) {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::CreateSnapshot {
            respond_to: send,
            snapshot_create_params,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn create_sync_action(&self, action: InstanceActionInfo) -> (String, String) {
        println!("ActorHandle CreateSyncAction Enter");
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::CreateSyncAction {
            respond_to: send,
            action,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn get_balloon(&self) -> Balloon {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::DescribeBalloonConfig { respond_to: send };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn get_balloon_stats(&self) -> BalloonStatistics {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::DescribeBalloonStats { respond_to: send };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn get_instance_info(&self) -> InstanceInfo {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::DescribeInstance { respond_to: send };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn get_vm_config(&self) -> FullVmConfiguration {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::GetExportVmConfig { respond_to: send };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn get_firecracker_version(&self) -> FirecrackerVersion {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::GetFirecrackerVersion { respond_to: send };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn get_machine_config(&self) -> MachineConfiguration {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::GetMachineConfiguration { respond_to: send };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn get_mmds(&self) -> String {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::GetMmds { respond_to: send };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn load_snapshot(
        &self,
        snapshot_load_params: SnapshotLoadParams,
    ) -> (String, String) {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::LoadSnapshot {
            respond_to: send,
            snapshot_load_params,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn patch_balloon_stats(
        &self,
        balloon_stats_update: BalloonStatsUpdate,
    ) -> (String, String) {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::PatchBalloonStatsInterval {
            respond_to: send,
            balloon_stats_update,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn patch_balloon(&self, balloon_update: BalloonUpdate) -> (String, String) {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::PatchBalloon {
            respond_to: send,
            balloon_update,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn patch_drive(&self, partial_drive: PartialDrive) -> (String, String) {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::PatchGuestDriveByID {
            respond_to: send,
            partial_drive,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn patch_network_interface(
        &self,
        partial_network_interface: PartialNetworkInterface,
    ) -> (String, String) {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::PatchGuestNetworkInterfaceByID {
            respond_to: send,
            partial_network_interface,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn patch_machine_config(
        &self,
        machine_config: MachineConfiguration,
    ) -> (String, String) {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::PatchMachineConfiguration {
            respond_to: send,
            machine_config,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn patch_mmds(&self, mmds_contents_object: MmdsContentsObject) -> (String, String) {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::PatchMmds {
            respond_to: send,
            mmds_contents_object,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn patch_vm(&self, vm: Vm) -> (String, String) {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::PatchVm {
            respond_to: send,
            vm,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn put_balloon(&self, balloon: Balloon) -> (String, String) {
        // status_code, status_text
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::PutBalloon {
            respond_to: send,
            balloon,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn put_cpu_config(&self, cpu_config: CPUConfig) -> (String, String) {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::PutCpuConfiguration {
            respond_to: send,
            cpu_config,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn put_entropy_device(&self, entropy_device: EntropyDevice) -> (String, String) {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::PutEntropyDevice {
            respond_to: send,
            entropy_device,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn put_guest_boot_source(&self, boot_source: BootSource) -> (String, String) {
        println!("ActorHandle PutGuestBootSource Enter");
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::PutGuestBootSource {
            respond_to: send,
            boot_source,
        };

        let _ = self.sender.send(msg).await;

        recv.await.expect("Actor task has been killed")
    }

    pub async fn put_drive(&self, drive: Drive) -> (String, String) {
        println!("ActorHandle PutGuestDriveByID Enter");
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::PutGuestDriveByID {
            respond_to: send,
            drive,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn put_network_interface(
        &self,
        network_interface: NetworkInterface,
    ) -> (String, String) {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::PutGuestNetworkInterfaceByID {
            respond_to: send,
            network_interface,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn put_vsock(&self, vsock: Vsock) -> (String, String) {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::PutGuestVsock {
            respond_to: send,
            vsock,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn put_logger(&self, logger: Logger) -> (String, String) {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::PutLogger {
            respond_to: send,
            logger,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn put_machine_config(
        &self,
        machine_config: MachineConfiguration,
    ) -> (String, String) {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::PutMachineConfiguration {
            respond_to: send,
            machine_config,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn put_metrics(&self, metrics: Metrics) -> (String, String) {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::PutMetrics {
            respond_to: send,
            metrics,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn put_mmds_config(&self, mmds_config: MmdsConfig) -> (String, String) {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::PutMmdsConfig {
            respond_to: send,
            mmds_config,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn put_mmds(&self, mmds_contents_object: MmdsContentsObject) -> (String, String) {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::PutMmds {
            respond_to: send,
            mmds_contents_object,
        };

        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }
}

/* 可以做两个, 一个是AsyncRead + AsyncWrite的Actor, 用于直接读写UnixStream */
/* 另一个是SendRequest方式的 */
/* 为了防止某一个Firecracker虚拟机的socket阻塞, 每一个FirecrackerClient的IO都要在一个单独的线程上进行异步任务启动 */

/* backpressure: 使用bounded通道, send在通道满载的时候会阻塞沉睡. 一旦两个actor互相发消息而且都backpressure, 那就有可能死锁 */
/* drop message: tokio的broadcast通道就是如此, 达到上限后丢弃更早的pending消息 */
/* kill the actor: 使用bounded通道和try_send, try_send不会等待, 一旦发送失败就直接返回错误 */
