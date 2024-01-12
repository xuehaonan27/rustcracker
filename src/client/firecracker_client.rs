use std::{path::PathBuf, os::unix::fs::FileTypeExt, fs, process::{Command, Child}, sync::{Arc, Mutex}};

use crate::{
    model::{
        balloon::Balloon, boot_source::BootSource, drive::Drive, error::InternalError,
        instance_action_info::InstanceActionInfo, logger::Logger,
        machine_configuration::MachineConfiguration, network_interface::NetworkInterface,
    },
    utils::Json,
};
use http_body_util::{BodyExt, Empty, Full};
use hyper::{
    body::{Body, Buf, Bytes},
    client::conn::http1::{Connection, SendRequest},
    header, Request,
};
use hyper_util::rt::TokioIo;
use serde::ser::StdError;
use tokio::{net::UnixStream, sync::Notify};

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
// type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

pub struct FirecrackerClient {
    socket_path: PathBuf,
    firecracker_binary_path: PathBuf,
}

impl FirecrackerClient {
    /* Getters */
    pub fn get_socket_path(&self) -> String {
        self.socket_path.to_string_lossy().into_owned()
    }

    /* 创建实例 */
    pub fn new<P>(socket_path: P, firecracker_binary_path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        Self {
            socket_path: socket_path.into(),
            firecracker_binary_path: firecracker_binary_path.into(),
        }
    }

    pub fn clear(&self) -> Result<()> {
        /* 若socket_path处已经有套接字文件则删除 */
        if let Ok(metadata) = fs::metadata(&self.socket_path) {
            if metadata.file_type().is_socket() {
                fs::remove_file(&self.socket_path)?;
            }
        }
        Ok(())
    }

    /* 启动firecracker(无jailer状态) */
    pub(crate) fn launch(&self) -> Result<Child> {
        /* sudo ${firecracker_binary_path} --api-sock "${socket_path}" */
        // let output = Command::new("sudo")
        //     .arg(&self.firecracker_binary_path)
        //     .arg("--api-sock")
        //     .arg(&self.socket_path)
        //     .output()?;

        // if !output.status.success() {
        //     return Err("Error when running firecracker".into())
        // }

        let child = Command::new("sudo")
            .arg(&self.firecracker_binary_path)
            .arg("--api-sock")
            .arg(&self.socket_path)
            .spawn()?;

        Ok(child)
    }

    /* 建立连接 */
    async fn establish_connection<B>(&self) -> Result<SendRequest<B>>
    where
        B: Body + 'static + Send,
        B::Data: Send,
        B::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        let stream = UnixStream::connect(&self.socket_path).await?;
        let io = TokioIo::new(stream);
        let (sender, conn): (SendRequest<B>, Connection<TokioIo<UnixStream>, B>) =
            hyper::client::conn::http1::handshake(io).await?;
        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                eprintln!("Connection failer: {:?}", err);
            }
        });

        Ok(sender)
    }

    pub(crate) async fn get_balloon(&self) -> Result<Balloon> {
        let mut sender = self.establish_connection().await?;
        let url: &'static str = "/balloon";
        let req = Request::get(url).body(Empty::<Bytes>::new())?;

        let res = sender.send_request(req).await?;

        let body = res.collect().await?.aggregate();

        let result = serde_json::from_reader::<_, Balloon>(body.reader())?;

        Ok(result)
    }

    pub(crate) async fn put_balloon(&self, balloon: Balloon) -> Result<()> {
        let mut sender = self.establish_connection().await?;
        let url: &'static str = "/balloon";
        let json = balloon.to_json()?;
        let length = json.as_bytes().len();
        let req = Request::put(url)
            .header(header::CONTENT_LENGTH, length)
            .body(Full::new(Bytes::from(json)))?;

        let res = sender.send_request(req).await?;

        let body = res.collect().await?.aggregate();

        let result = serde_json::from_reader::<_, InternalError>(body.reader());
        match result {
            Ok(_) => return Err("putBalloon".into()),
            Err(_) => return Ok(()),
        }
    }

    pub async fn put_guest_boot_source(&self, boot_source: BootSource) -> Result<()> {
        let mut sender = self.establish_connection().await?;
        let url: &'static str = "/boot-source";
        let json = boot_source.to_json()?;
        let length = json.as_bytes().len();
        let req = Request::put(url)
            .header(header::CONTENT_LENGTH, length)
            .body(Full::new(Bytes::from(json)))?;

        let res = sender.send_request(req).await?;

        let body = res.collect().await?.aggregate();

        let result = serde_json::from_reader::<_, InternalError>(body.reader());
        match result {
            Ok(_) => return Err("putGuestBootSource".into()),
            Err(_) => return Ok(()),
        }
    }

    pub(crate) async fn put_guest_drive_by_id(&self, drive: Drive) -> Result<()> {
        let mut sender = self.establish_connection().await?;

        let json = drive.to_json()?;
        let length = json.as_bytes().len();
        let drive_id = drive.get_drive_id();

        let url = format!("/drives/{drive_id}");

        let req = Request::put(url)
            .header(header::CONTENT_LENGTH, length)
            .body(Full::new(Bytes::from(json)))?;

        let res = sender.send_request(req).await?;

        let body = res.collect().await?.aggregate();

        let result = serde_json::from_reader::<_, InternalError>(body.reader());
        match result {
            Ok(_) => return Err("putGuestDriveByID".into()),
            Err(_) => return Ok(()),
        }
    }

    pub(crate) async fn put_machine_configuration(
        &self,
        machine_config: MachineConfiguration,
    ) -> Result<()> {
        let mut sender = self.establish_connection().await?;

        let url: &'static str = "/machine-config";
        let json = machine_config.to_json()?;
        let length = json.as_bytes().len();

        let req = Request::put(url)
            .header(header::CONTENT_LENGTH, length)
            .body(Full::new(Bytes::from(json)))?;

        let res = sender.send_request(req).await?;

        let body = res.collect().await?.aggregate();
        let result = serde_json::from_reader::<_, InternalError>(body.reader());

        match result {
            Ok(_) => return Err("putMachineConfiguration".into()),
            Err(_) => return Ok(()),
        }
    }

    pub(crate) async fn put_logger(&self, logger: Logger) -> Result<()> {
        let mut sender = self.establish_connection().await?;

        let url: &'static str = "/logger";
        let json = logger.to_json()?;
        let length = json.as_bytes().len();

        let req = Request::put(url)
            .header(header::CONTENT_LENGTH, length)
            .body(Full::new(Bytes::from(json)))?;

        let res = sender.send_request(req).await?;

        let body = res.collect().await?.aggregate();

        let result = serde_json::from_reader::<_, InternalError>(body.reader());
        match result {
            Ok(_) => return Err("putLogger".into()),
            Err(_) => return Ok(()),
        }
    }

    pub(crate) async fn put_guest_network_interface_by_id(
        &self,
        network_interface: NetworkInterface,
    ) -> Result<()> {
        let mut sender = self.establish_connection().await?;

        let json = network_interface.to_json()?;
        let length = json.as_bytes().len();
        let iface_id = network_interface.get_iface_id();

        let url = format!("/network-interfaces/{iface_id}");

        let req = Request::put(url)
            .header(header::CONTENT_LENGTH, length)
            .body(Full::new(Bytes::from(json)))?;

        let res = sender.send_request(req).await?;

        let body = res.collect().await?.aggregate();

        let result = serde_json::from_reader::<_, InternalError>(body.reader());

        match result {
            Ok(_) => return Err("putGuestNetworkInterfaceByID".into()),
            Err(_) => return Ok(()),
        }
    }

    pub(crate) async fn create_sync_action(&self, action: InstanceActionInfo) -> Result<()> {
        let mut sender = self.establish_connection().await?;

        let url: &'static str = "actions";

        let json = action.to_json()?;
        let length = json.as_bytes().len();

        let req = Request::put(url)
            .header(header::CONTENT_LENGTH, length)
            .body(Full::new(Bytes::from(json)))?;

        let res = sender.send_request(req).await?;

        let body = res.collect().await?.aggregate();
        let result = serde_json::from_reader::<_, InternalError>(body.reader());

        match result {
            Ok(_) => return Err("createSyncAction".into()),
            Err(_) => return Ok(()),
        }
    }
}

pub async fn demo(client_arcmutex: Arc<Mutex<FirecrackerClient>>, notify_ptr: Option<Arc<Notify>>, machine_config: MachineConfiguration) -> Result<()> {
    let client = client_arcmutex.lock().unwrap();
    
    if let Some(notify_ptr) = notify_ptr {
        notify_ptr.notified();
    }

    client.put_machine_configuration(machine_config).await?;
    
    #[cfg(debug_assertions)]
    println!(
        "rustfire:main[{}]: machine configuration specified",
        line!()
    );
    Ok(())
}

pub async fn launch(client_arcmutex: Arc<Mutex<FirecrackerClient>>, notify_ptr: Option<Arc<Notify>>, timeout_secs: u64) -> Result<Child> {
    if notify_ptr.is_none() {
        return Err("Need a notifier to sync with firecracker".into());
    }
    let client = client_arcmutex.lock().unwrap();

    client.clear()?;
    let child = client.launch()?;

    let result = tokio::time::timeout(tokio::time::Duration::from_secs(timeout_secs), async move {
        while let Err(_) = tokio::fs::metadata(client.get_socket_path()).await {}
    }).await;
    
    match result {
        Ok(_) => {
            notify_ptr.unwrap().notify_waiters();
            return Ok(child)
        },
        Err(_) => {
            eprintln!("Timeout after {timeout_secs} secs when waiting the firecracker");
            return Err("Timeout".into());
        },
    }
}

pub async fn get_balloon(client_arcmutex: Arc<Mutex<FirecrackerClient>>, notify_ptr: Option<Arc<Notify>>) -> Result<Balloon> {
    let client = client_arcmutex.lock().unwrap();
    
    if let Some(notify_ptr) = notify_ptr {
        notify_ptr.notified();
    }

    let result = client.get_balloon().await?;
    
    #[cfg(debug_assertions)]
    println!(
        "rustfire:main[{}]: balloon got: {:#?}",
        line!(), result
    );
    Ok(result)
}

pub async fn put_balloon(client_arcmutex: Arc<Mutex<FirecrackerClient>>, notify_ptr: Option<Arc<Notify>>, balloon: Balloon) -> Result<()> {
    let client = client_arcmutex.lock().unwrap();
    
    if let Some(notify_ptr) = notify_ptr {
        notify_ptr.notified();
    }

    client.put_balloon(balloon).await?;

    #[cfg(debug_assertions)]
    println!(
        "rustfire:main[{}]: balloon put",
        line!(),
    );
    Ok(())
}

pub async fn put_guest_boot_source(client_arcmutex: Arc<Mutex<FirecrackerClient>>, notify_ptr: Option<Arc<Notify>>, boot_source: BootSource) -> Result<()> {
    let client = client_arcmutex.lock().unwrap();
    
    if let Some(notify_ptr) = notify_ptr {
        notify_ptr.notified();
    }

    client.put_guest_boot_source(boot_source).await?;

    #[cfg(debug_assertions)]
    println!(
        "rustfire:main[{}]: guest boot source put",
        line!()
    );
    Ok(())
}

pub async fn put_guest_drive_by_id(client_arcmutex: Arc<Mutex<FirecrackerClient>>, notify_ptr: Option<Arc<Notify>>, drive: Drive) -> Result<()> {
    let client = client_arcmutex.lock().unwrap();
    
    if let Some(notify_ptr) = notify_ptr {
        notify_ptr.notified();
    }

    client.put_guest_drive_by_id(drive).await?;

    #[cfg(debug_assertions)]
    println!(
        "rustfire:main[{}]: guest drive put",
        line!()
    );
    Ok(())   
}

pub async fn put_machine_configuration(client_arcmutex: Arc<Mutex<FirecrackerClient>>, notify_ptr: Option<Arc<Notify>>, machine_config: MachineConfiguration) -> Result<()> {
    let client = client_arcmutex.lock().unwrap();
    
    if let Some(notify_ptr) = notify_ptr {
        notify_ptr.notified();
    }

    client.put_machine_configuration(machine_config).await?;

    #[cfg(debug_assertions)]
    println!(
        "rustfire:main[{}]: guest drive put",
        line!()
    );
    Ok(())
}

pub async fn put_logger(client_arcmutex: Arc<Mutex<FirecrackerClient>>, notify_ptr: Option<Arc<Notify>>, logger: Logger) -> Result<()> {
    let client = client_arcmutex.lock().unwrap();

    if let Some(notify_ptr) = notify_ptr {
        notify_ptr.notified();
    }

    client.put_logger(logger).await?;

    #[cfg(debug_assertions)]
    println!(
        "rustfire:main[{}]: logger put",
        line!()
    );
    Ok(())
}

pub async fn put_guest_network_interface_by_id(client_arcmutex: Arc<Mutex<FirecrackerClient>>, notify_ptr: Option<Arc<Notify>>, network_interface: NetworkInterface) -> Result<()> {
    let client = client_arcmutex.lock().unwrap();

    if let Some(notify_ptr) = notify_ptr {
        notify_ptr.notified();
    }

    client.put_guest_network_interface_by_id(network_interface).await?;

    #[cfg(debug_assertions)]
    println!(
        "rustfire:main[{}]: network interface put",
        line!()
    );
    Ok(())
}

pub async fn create_sync_action(client_arcmutex: Arc<Mutex<FirecrackerClient>>, notify_ptr: Option<Arc<Notify>>, action: InstanceActionInfo) -> Result<()> {
    let client = client_arcmutex.lock().unwrap();

    if let Some(notify_ptr) = notify_ptr {
        notify_ptr.notified();
    }

    client.create_sync_action(action).await?;

    #[cfg(debug_assertions)]
    println!(
        "rustfire:main[{}]: action created",
        line!()
    );
    Ok(())
}