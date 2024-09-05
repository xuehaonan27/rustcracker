use crate::config::HypervisorConfig;
use crate::handle_entry;
use crate::jailer::JailerSync;
use crate::{Error, Result};
use log::*;
use serde::{Deserialize, Serialize};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

/// Unlike using jailer, when using bare firecracker, socket path and lock path must be specified
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FirecrackerSync {
    pub(crate) id: String,

    // Path to local firecracker bin
    // Usually something like `/usr/bin/firecracker` if not using jailer
    pub(crate) bin: String,

    pub(crate) socket: PathBuf,

    pub(crate) lock_path: PathBuf,

    pub(crate) log_path: Option<PathBuf>,

    // Path to the config file
    pub(crate) config_path: Option<String>,
}

impl FirecrackerSync {
    /// Using bare firecracker
    pub(crate) fn from_config(config: &HypervisorConfig) -> Result<Self> {
        let id = if let Some(id) = &config.id {
            id.clone()
        } else {
            uuid::Uuid::new_v4().to_string()
        };

        let socket = if let Some(socket) = &config.socket_path {
            socket.clone()
        } else {
            // allocate one. format: /run/firecracker-<id>.socket
            format!("/run/firecracker-{}.socket", id)
        };
        let socket = PathBuf::from(socket);

        Ok(Self {
            id,
            bin: handle_entry(&config.frck_bin, "firecracker binary")?,
            socket,
            lock_path: handle_entry(&config.lock_path, "lock path")?.into(),
            log_path: config.log_path.clone().map(PathBuf::from),
            config_path: config.frck_export_path.clone().and_then(|s| Some(s.into())),
        })
    }

    /// Using firecracker with jailer
    pub(crate) fn from_jailer(jailer: JailerSync) -> Result<Self> {
        let bin = jailer.get_firecracker_exec_file().clone();

        let socket = jailer
            .get_socket_path_exported()
            .ok_or_else(|| {
                let msg = "Jailer without exported socket path";
                error!("{msg}");
                Error::Config(msg.into())
            })?
            .clone();

        let lock_path = jailer
            .get_lock_path_exported()
            .ok_or_else(|| {
                let msg = "Jailer without exported lock path";
                error!("{msg}");
                Error::Config(msg.into())
            })?
            .clone();

        let log_path = jailer
            .get_log_path_exported()
            .ok_or_else(|| {
                let msg = "Jailer without exported log path";
                error!("{msg}");
                Error::Config(msg.into())
            })?
            .clone();

        let config_path = jailer.get_config_path_exported().cloned();

        Ok(Self {
            id: jailer.id,
            bin,
            socket,
            lock_path,
            log_path: Some(log_path),
            config_path,
        })
    }

    pub(crate) fn launch(&self) -> Result<std::process::Child> {
        let mut c = std::process::Command::new(&self.bin);
        let mut c = c.arg("--api-sock").arg(&self.socket);
        match &self.config_path {
            Some(config_path) => c = c.arg("--config-file").arg(&config_path),
            None => (),
        }

        c.spawn().map_err(|e| {
            let msg = format!("Fail to spawn firecracker process: {e}");
            error!("{msg}");
            Error::Firecracker(msg)
        })
    }

    /// Waiting for the socket set by firecracker
    pub(crate) fn waiting_socket(&self, timeout: std::time::Duration) -> Result<()> {
        let start = std::time::Instant::now();
        let socket_path = PathBuf::from(&self.socket);
        while start.elapsed() < timeout {
            if socket_path.exists() {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(100)); // check every 100 ms
        }

        Err(Error::Jailer("remote socket timeout".to_string()))
    }

    /// Connect to the socket
    pub(crate) fn connect(&self, retry: usize) -> Result<UnixStream> {
        let mut trying = retry;
        let stream = loop {
            if trying == 0 {
                return Err(Error::Firecracker(format!(
                    "Fail to connect unix socket after {retry} tries"
                )));
            }
            match UnixStream::connect(&self.socket) {
                Ok(stream) => break stream,
                Err(_) => {
                    trying -= 1;
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
            }
        };
        Ok(stream)
    }

    /// Get socket path
    pub fn get_socket_path(&self) -> PathBuf {
        self.socket.clone()
    }
}
