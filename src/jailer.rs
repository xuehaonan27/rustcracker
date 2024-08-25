pub mod jailer {
    use crate::config::HypervisorConfig;
    use crate::{handle_entry, handle_entry_default, handle_entry_ref};
    use crate::{RtckError, RtckResult};
    use log::*;
    use serde::{Deserialize, Serialize};
    use std::{os::unix::net::UnixStream, path::PathBuf};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Jailer {
        // Path to local jailer bin
        // Usually something like `/usr/bin/jailer`
        bin: String,

        // Id of this jailer
        pub(crate) id: String,

        // Path to local firecracker bin
        exec_file: String,

        // Uid
        uid: u32,

        // Gid
        gid: u32,

        // chroot base directory, default to /srv/jailer
        chroot_base_dir: String,

        // Daemonize or not
        daemonize: bool,

        // Jailer workspace directory
        jailer_workspace_dir: Option<PathBuf>,

        // Desired path of the socket
        socket: Option<String>,

        // Socket path seen by Rtck
        socket_path_export: Option<PathBuf>,

        // Desired path of the lock
        lock_path: Option<String>,

        // Lock path seen by Rtck
        lock_path_export: Option<PathBuf>,

        // Path to the config file
        config_path: Option<String>,

        // Config file path seen by firecracker
        config_path_jailed: Option<PathBuf>,

        // Log path seen by firecracker
        log_path: Option<String>,

        // Log path seen by Rtck
        log_path_export: Option<PathBuf>,

        // Metrics path seen by firecracker
        metrics_path: Option<String>,

        // Metrics path seen by Rtck
        metrics_path_export: Option<PathBuf>,
    }

    impl Jailer {
        pub fn get_uid(&self) -> u32 {
            self.uid
        }

        pub fn get_gid(&self) -> u32 {
            self.gid
        }

        pub fn get_firecracker_exec_file(&self) -> &String {
            &self.exec_file
        }

        pub fn get_socket_path_exported(&self) -> Option<&PathBuf> {
            self.socket_path_export.as_ref()
        }

        pub fn get_lock_path_exported(&self) -> Option<&PathBuf> {
            self.lock_path_export.as_ref()
        }

        pub fn get_log_path_exported(&self) -> Option<&PathBuf> {
            self.log_path_export.as_ref()
        }

        pub fn get_metrics_path_exported(&self) -> Option<&PathBuf> {
            self.metrics_path_export.as_ref()
        }

        pub fn get_config_path_exported(&self) -> Option<&String> {
            self.config_path.as_ref()
        }

        pub fn get_jailer_workspace_dir(&self) -> Option<&PathBuf> {
            self.jailer_workspace_dir.as_ref()
        }
    }

    impl Jailer {
        pub fn from_config(config: &HypervisorConfig) -> RtckResult<Self> {
            config.validate()?;

            let jailer_config = config.jailer_config.as_ref().ok_or_else(|| {
                let msg = "Missing jailer config";
                error!("{msg}");
                RtckError::Config(msg.into())
            })?;

            let id = if let Some(id) = &config.id {
                id.clone()
            } else {
                uuid::Uuid::new_v4().to_string()
            };

            let socket = if let Some(socket) = &config.socket_path {
                socket.clone()
            } else {
                // allocate one. format: run/firecracker.socket
                "run/firecracker.socket".to_string()
            };
            let socket = Some(socket);

            const DEFAULT_CHROOT_BASE_DIR: &'static str = "/srv/jailer";
            Ok(Self {
                bin: handle_entry(&jailer_config.jailer_bin, "jailer binary")?,
                id,
                exec_file: handle_entry(&jailer_config.exec_file, "firecracker executable file")?,
                uid: handle_entry(&jailer_config.uid, "jailer uid")?,
                gid: handle_entry(&jailer_config.gid, "jailer gid")?,
                chroot_base_dir: handle_entry_default(
                    &jailer_config.chroot_base_dir,
                    DEFAULT_CHROOT_BASE_DIR.into(),
                ),
                daemonize: jailer_config.daemonize.unwrap_or(false),
                jailer_workspace_dir: None,
                socket,
                socket_path_export: None,
                lock_path: config.lock_path.clone(),
                lock_path_export: None,
                config_path: config.frck_export_path.clone(),
                config_path_jailed: None,
                log_path: config.log_path.clone(),
                log_path_export: None,
                metrics_path: config.metrics_path.clone(),
                metrics_path_export: None,
            })
        }

        pub fn jail(&mut self) -> RtckResult<PathBuf> {
            let id = &self.id;
            let temp_binding = PathBuf::from(&self.exec_file);
            let exec_file_name =
                *handle_entry_ref(&temp_binding.file_name(), "firecracker executable file")?;
            let chroot_base_dir = &self.chroot_base_dir;
            const ROOT_FOLDER_NAME: &'static str = "root";
            let instance_dir = PathBuf::from(chroot_base_dir).join(exec_file_name).join(id);
            let jailer_workspace_dir = instance_dir.join(ROOT_FOLDER_NAME);
            if jailer_workspace_dir.exists() {
                let msg = "Conflict instance name, please choose another one";
                error!("{msg}");
                return Err(RtckError::Jailer(msg.into()));
            }
            self.jailer_workspace_dir = Some(jailer_workspace_dir.clone());

            // Get socket path under jailer
            const DEFAULT_SOCKET_PATH_UNDER_JAILER: &'static str = "run/firecracker.socket";
            self.socket_path_export = self.get_x_path_under_jailer(
                &self.socket,
                DEFAULT_SOCKET_PATH_UNDER_JAILER,
                "socket",
                &jailer_workspace_dir,
            )?;

            // Get lock path under jailer
            const DEFAULT_LOCK_PATH_UNDER_JAILER: &'static str = "run/firecracker.lock";
            self.lock_path_export = self.get_x_path_under_jailer(
                &self.lock_path,
                DEFAULT_LOCK_PATH_UNDER_JAILER,
                "lock",
                &jailer_workspace_dir,
            )?;

            // Get log path under jailer
            const DEFAULT_LOG_PATH_UNDER_JAILER: &'static str = "run/firecracker.log";
            self.log_path_export = self.get_x_path_under_jailer(
                &self.log_path,
                DEFAULT_LOG_PATH_UNDER_JAILER,
                "log",
                &jailer_workspace_dir,
            )?;

            // Get metrics path under jailer
            const DEFAULT_METRICS_PATH_UNDER_JAILER: &'static str = "run/firecracker.metrics";
            self.metrics_path_export = self.get_x_path_under_jailer(
                &self.metrics_path,
                DEFAULT_METRICS_PATH_UNDER_JAILER,
                "metrics",
                &jailer_workspace_dir,
            )?;

            match &self.config_path {
                // not using config exported config, skipping
                None => (),
                Some(config_path) => {
                    // copy the config file into the jailer
                    const DEFAULT_CONFIG_PATH_JAILED: &'static str = "run/firecracker-config.json";
                    self.config_path_jailed = Some(DEFAULT_CONFIG_PATH_JAILED.into());

                    let config_path_export = jailer_workspace_dir.join(DEFAULT_CONFIG_PATH_JAILED);
                    std::fs::copy(config_path, config_path_export).map_err(|e| {
                        let msg = format!("Fail to copy config when jailing: {e}");
                        error!("{msg}");
                        RtckError::FilesysIO(msg)
                    })?;
                }
            }

            Ok(instance_dir)
        }

        fn get_x_path_under_jailer(
            &self,
            x: &Option<String>,
            default_path_under_jailer: &'static str,
            name: &'static str,
            jailer_workspace_dir: &PathBuf,
        ) -> RtckResult<Option<PathBuf>> {
            let path = handle_entry_default(x, default_path_under_jailer.to_string());
            let path = PathBuf::from(path);
            let path = if path.is_absolute() {
                path.strip_prefix("/").map_err(|e| {
                    let msg = format!("Fail to strip prefix of {name} path when jailing: {e}");
                    error!("{msg}");
                    RtckError::Jailer(msg)
                })?
            } else {
                path.as_path()
            };
            Ok(Some(jailer_workspace_dir.join(path)))
        }

        pub fn launch(&self) -> RtckResult<std::process::Child> {
            let mut cmd = std::process::Command::new(&self.bin);
            cmd.args(vec!["--id", &self.id]);
            cmd.args(vec!["--uid", &self.uid.to_string()]);
            cmd.args(vec!["--gid", &self.gid.to_string()]);
            cmd.args(vec!["--exec-file", &self.exec_file.to_string()]);
            cmd.args(vec!["--chroot-base-dir", &self.chroot_base_dir]);
            if self.daemonize {
                cmd.arg("--daemonize");
            }
            cmd.arg("--");

            match &self.socket {
                None => (),
                Some(path) => {
                    cmd.args(vec!["--api-sock", path]);
                }
            }

            match &self.config_path {
                None => (),
                Some(path) => {
                    cmd.args(vec!["--config-file", path]);
                }
            }

            cmd.spawn().map_err(|e| {
                let msg = format!("Fail to spawn jailer: {e}");
                error!("{msg}");
                RtckError::Jailer(msg)
            })
        }

        /// Waiting for the socket set by firecracker
        pub fn waiting_socket(&self, timeout: std::time::Duration) -> RtckResult<()> {
            let start = std::time::Instant::now();
            let socket_path = handle_entry(&self.socket_path_export, "exported socket path")?;
            while start.elapsed() < timeout {
                if socket_path.exists() {
                    return Ok(());
                }
                std::thread::sleep(std::time::Duration::from_millis(100)); // check every 100 ms
            }

            Err(RtckError::Jailer("Remote socket timeout".to_string()))
        }

        /// Connect to the socket
        pub fn connect(&self, retry: usize) -> RtckResult<UnixStream> {
            let mut trying = retry;
            let socket = handle_entry_ref(&self.socket_path_export, "exported socket path")?;
            let stream = loop {
                if trying == 0 {
                    return Err(RtckError::Firecracker(format!(
                        "Fail to connect unix socket after {retry} tries"
                    )));
                }
                match UnixStream::connect(socket) {
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
    }
}

pub mod jailer_async {
    use crate::config::HypervisorConfig;
    use crate::{handle_entry, handle_entry_default, handle_entry_ref};
    use crate::{RtckError, RtckResult};
    use log::*;
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;
    use tokio::net::UnixStream;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct JailerAsync {
        // Path to local jailer bin
        // Usually something like `/usr/bin/jailer`
        bin: String,

        // Id of this jailer
        pub(crate) id: String,

        // Path to local firecracker bin
        exec_file: String,

        // Uid
        uid: u32,

        // Gid
        gid: u32,

        // chroot base directory, default to /srv/jailer
        chroot_base_dir: String,

        // Daemonize or not
        daemonize: bool,

        // Jailer workspace directory
        jailer_workspace_dir: Option<PathBuf>,

        // Desired path of the socket
        socket: Option<String>,

        // Socket path seen by Rtck
        socket_path_export: Option<PathBuf>,

        // Desired path of the lock
        lock_path: Option<String>,

        // Lock path seen by Rtck
        lock_path_export: Option<PathBuf>,

        // Path to the config file
        config_path: Option<String>,

        // Config file path seen by firecracker
        config_path_jailed: Option<PathBuf>,

        // Log path seen by firecracker
        log_path: Option<String>,

        // Log path seen by Rtck
        log_path_export: Option<PathBuf>,

        // Metrics path seen by firecracker
        metrics_path: Option<String>,

        // Metrics path seen by Rtck
        metrics_path_export: Option<PathBuf>,
    }

    impl JailerAsync {
        pub fn get_uid(&self) -> u32 {
            self.uid
        }

        pub fn get_gid(&self) -> u32 {
            self.gid
        }

        pub fn get_firecracker_exec_file(&self) -> &String {
            &self.exec_file
        }

        pub fn get_socket_path_exported(&self) -> Option<&PathBuf> {
            self.socket_path_export.as_ref()
        }

        pub fn get_lock_path_exported(&self) -> Option<&PathBuf> {
            self.lock_path_export.as_ref()
        }

        pub fn get_log_path_exported(&self) -> Option<&PathBuf> {
            self.log_path_export.as_ref()
        }

        pub fn get_metrics_path_exported(&self) -> Option<&PathBuf> {
            self.metrics_path_export.as_ref()
        }

        pub fn get_config_path_exported(&self) -> Option<&String> {
            self.config_path.as_ref()
        }

        pub fn get_jailer_workspace_dir(&self) -> Option<&PathBuf> {
            self.jailer_workspace_dir.as_ref()
        }
    }

    impl JailerAsync {
        pub fn from_config(config: &HypervisorConfig) -> RtckResult<Self> {
            let jailer_config = config.jailer_config.as_ref();
            let jailer_config = match jailer_config {
                Some(c) => c,
                None => {
                    let msg = "Missing jailer config";
                    error!("{msg}");
                    return Err(RtckError::Config(msg.into()));
                }
            };

            // Fetch from configuration, or allocate one if not present.
            let id = config.id.clone().unwrap_or({
                info!("`id` not assigned in jailer configuration, allocating a random one");
                uuid::Uuid::new_v4().into()
            });
            let socket = config.socket_path.clone().unwrap_or({
                info!("`socket` not assigned in jailer configuration, allocating a random one");
                "run/firecracker.socket".into()
            });

            const DEFAULT_CHROOT_BASE_DIR: &'static str = "/srv/jailer";
            Ok(Self {
                bin: handle_entry(&jailer_config.jailer_bin, "jailer binary")?,
                id,
                exec_file: handle_entry(&jailer_config.exec_file, "firecracker executable file")?,
                uid: handle_entry(&jailer_config.uid, "jailer uid")?,
                gid: handle_entry(&jailer_config.gid, "jailer gid")?,
                chroot_base_dir: handle_entry_default(
                    &jailer_config.chroot_base_dir,
                    DEFAULT_CHROOT_BASE_DIR.into(),
                ),
                daemonize: jailer_config.daemonize.unwrap_or(false),
                jailer_workspace_dir: None,
                socket: Some(socket),
                socket_path_export: None,
                lock_path: config.lock_path.clone(),
                lock_path_export: None,
                config_path: config.frck_export_path.clone(),
                config_path_jailed: None,
                log_path: config.log_path.clone(),
                log_path_export: None,
                metrics_path: config.metrics_path.clone(),
                metrics_path_export: None,
            })
        }

        pub async fn jail(&mut self) -> RtckResult<PathBuf> {
            // Get jailer workspace directory
            let id = &self.id;
            let temp_binding = PathBuf::from(&self.exec_file);
            let exec_file_name =
                *handle_entry_ref(&temp_binding.file_name(), "firecracker executable file")?;
            let chroot_base_dir = &self.chroot_base_dir;
            const ROOT_FOLDER_NAME: &'static str = "root";
            let instance_dir = PathBuf::from(chroot_base_dir).join(exec_file_name).join(id);
            let jailer_workspace_dir = instance_dir.join(ROOT_FOLDER_NAME);
            if jailer_workspace_dir.exists() {
                let msg = "Conflict instance name, please choose another one";
                error!("{msg}");
                return Err(RtckError::Jailer(msg.into()));
            }
            self.jailer_workspace_dir = Some(jailer_workspace_dir.clone());

            // Get socket path under jailer
            const DEFAULT_SOCKET_PATH_UNDER_JAILER: &'static str = "run/firecracker.socket";
            self.socket_path_export = self.get_x_path_under_jailer(
                &self.socket,
                DEFAULT_SOCKET_PATH_UNDER_JAILER,
                "socket",
                &jailer_workspace_dir,
            )?;

            // Get lock path under jailer
            const DEFAULT_LOCK_PATH_UNDER_JAILER: &'static str = "run/firecracker.lock";
            self.lock_path_export = self.get_x_path_under_jailer(
                &self.lock_path,
                DEFAULT_LOCK_PATH_UNDER_JAILER,
                "lock",
                &jailer_workspace_dir,
            )?;

            // Get log path under jailer
            const DEFAULT_LOG_PATH_UNDER_JAILER: &'static str = "run/firecracker.log";
            self.log_path_export = self.get_x_path_under_jailer(
                &self.log_path,
                DEFAULT_LOG_PATH_UNDER_JAILER,
                "log",
                &jailer_workspace_dir,
            )?;

            // Get metrics path under jailer
            const DEFAULT_METRICS_PATH_UNDER_JAILER: &'static str = "run/firecracker.metrics";
            self.metrics_path_export = self.get_x_path_under_jailer(
                &self.metrics_path,
                DEFAULT_METRICS_PATH_UNDER_JAILER,
                "metrics",
                &jailer_workspace_dir,
            )?;

            match &self.config_path {
                // not using config exported config, skipping
                None => (),
                Some(config_path) => {
                    // copy the config file into the jailer
                    const DEFAULT_CONFIG_PATH_JAILED: &'static str = "run/firecracker-config.json";
                    self.config_path_jailed = Some(DEFAULT_CONFIG_PATH_JAILED.into());

                    let config_path_export = jailer_workspace_dir.join(DEFAULT_CONFIG_PATH_JAILED);
                    tokio::fs::copy(config_path, config_path_export)
                        .await
                        .map_err(|e| {
                            let msg = format!("Fail to copy config when jailing: {e}");
                            error!("{msg}");
                            RtckError::FilesysIO(msg)
                        })?;
                }
            }

            Ok(instance_dir)
        }

        fn get_x_path_under_jailer(
            &self,
            x: &Option<String>,
            default_path_under_jailer: &'static str,
            name: &'static str,
            jailer_workspace_dir: &PathBuf,
        ) -> RtckResult<Option<PathBuf>> {
            let path = handle_entry_default(x, default_path_under_jailer.to_string());
            let path = PathBuf::from(path);
            let path = if path.is_absolute() {
                path.strip_prefix("/").map_err(|e| {
                    let msg = format!("Fail to strip prefix of {name} path when jailing: {e}");
                    error!("{msg}");
                    RtckError::Jailer(msg)
                })?
            } else {
                path.as_path()
            };
            Ok(Some(jailer_workspace_dir.join(path)))
        }

        pub async fn launch(&self) -> RtckResult<tokio::process::Child> {
            let mut cmd = tokio::process::Command::new(&self.bin);
            cmd.args(vec!["--id", &self.id]);
            cmd.args(vec!["--uid", &self.uid.to_string()]);
            cmd.args(vec!["--gid", &self.gid.to_string()]);
            cmd.args(vec!["--exec-file", &self.exec_file.to_string()]);
            cmd.args(vec!["--chroot-base-dir", &self.chroot_base_dir]);
            if self.daemonize {
                cmd.arg("--daemonize");
            }
            cmd.arg("--");

            match &self.socket {
                None => (),
                Some(path) => {
                    cmd.args(vec!["--api-sock", path]);
                }
            }

            match &self.config_path {
                None => (),
                Some(path) => {
                    cmd.args(vec!["--config-file", path]);
                }
            }

            cmd.spawn().map_err(|e| {
                let msg = format!("Fail to spawn jailer: {e}");
                error!("{msg}");
                RtckError::Jailer(msg)
            })
        }

        /// Waiting for the socket set by firecracker
        pub async fn waiting_socket(&self, timeout: tokio::time::Duration) -> RtckResult<()> {
            let socket_path = handle_entry(&self.socket_path_export, "exported socket path")?;
            let socket_path = socket_path.as_os_str();
            tokio::time::timeout(timeout, async {
                while tokio::fs::try_exists(socket_path).await.is_err() {}
            })
            .await
            .map_err(|e| {
                let msg = format!("Remote socket timeout: {e}");
                error!("{msg}");
                RtckError::Jailer(msg)
            })
        }

        /// Connect to the socket
        pub async fn connect(&self, retry: usize) -> RtckResult<UnixStream> {
            let mut trying = retry;
            let socket = handle_entry_ref(&self.socket_path_export, "exported socket path")?;
            let stream = loop {
                if trying == 0 {
                    let msg = format!("fail to connect unix socket after {retry} tries");
                    error!("{msg}");
                    return Err(RtckError::Firecracker(msg));
                }
                match UnixStream::connect(socket).await {
                    Ok(stream) => break stream,
                    Err(_) => {
                        trying -= 1;
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        continue;
                    }
                }
            };
            Ok(stream)
        }
    }
}

pub use jailer::Jailer;
pub use jailer_async::JailerAsync;
