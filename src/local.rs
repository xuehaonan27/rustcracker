use crate::{RtckError, RtckErrorClass, RtckResult};

/// Module for manipulating host firecracker process
pub mod firecracker {

    use std::{path::PathBuf, sync::Arc};

    use parking_lot::{Condvar, Mutex};

    use crate::{config::GlobalConfig, RtckError, RtckErrorClass, RtckResult};

    use super::handle_entry;

    pub struct Firecracker {
        // Path to local firecracker bin
        // Usually something like `/usr/bin/firecracker` if not using jailer
        bin: String,

        // Desired path of the socket
        socket: String,

        // Path to the config file
        config_path: Option<String>,
    }

    impl Firecracker {
        pub fn get_socket(&self) -> &String {
            &self.socket
        }
    }

    impl Firecracker {
        pub fn from_config(config: &GlobalConfig) -> RtckResult<Self> {
            config.validate()?;

            Ok(Self {
                bin: handle_entry(&config.frck_bin)?,
                socket: handle_entry(&config.socket_path)?,
                config_path: config.frck_export_path.clone(),
            })
        }

        pub fn launch(&self) -> RtckResult<std::process::Child> {
            let mut c = std::process::Command::new(&self.bin);
            let mut c = c.arg("--api-sock").arg(&self.socket);
            match &self.config_path {
                Some(config_path) => c = c.arg("--config-file").arg(&config_path),
                None => (),
            }
            Ok(c.spawn()?)
        }

        /// Waiting for the socket set by firecracker
        pub fn waiting_socket(&self, timeout: std::time::Duration) -> RtckResult<()> {
            let pair = Arc::new((Mutex::new(false), Condvar::new()));
            let pair_peer = Arc::clone(&pair);

            // Wait for the socket
            let path = PathBuf::from(&self.socket);
            std::thread::spawn(move || {
                let &(ref lock, ref cvar) = &*pair_peer;
                let mut created = lock.lock();

                while !path.exists() {}

                *created = true;
                cvar.notify_one();
            });

            let &(ref lock, ref cvar) = &*pair;
            let created = lock.lock();
            if !*created {
                let result = cvar.wait_for(&mut lock.lock(), timeout);
                if result.timed_out() {
                    return Err(RtckError::new(
                        RtckErrorClass::RemoteError,
                        "Remote socket set up timeout".to_string(),
                    ));
                }
            }

            Ok(())
        }

        /// Connect to the socket
        pub fn connect(&self) -> RtckResult<bufstream::BufStream<std::os::unix::net::UnixStream>> {
            Ok(bufstream::BufStream::new(
                std::os::unix::net::UnixStream::connect(&self.socket)?,
            ))
        }
    }
}

pub mod firecracker_async {
    use crate::{config::GlobalConfig, RtckError, RtckErrorClass, RtckResult};

    use super::handle_entry;

    pub struct FirecrackerAsync {
        // Path to local firecracker bin
        // Usually something like `/usr/bin/firecracker` if not using jailer
        bin: String,

        // Desired path of the socket
        socket: String,

        // Path to the config file
        config_path: Option<String>,
    }

    impl FirecrackerAsync {
        pub fn get_socket(&self) -> &String {
            &self.socket
        }
    }

    impl FirecrackerAsync {
        pub fn from_config(config: &GlobalConfig) -> RtckResult<Self> {
            Ok(Self {
                bin: handle_entry(&config.frck_bin)?,
                socket: handle_entry(&config.socket_path)?,
                config_path: config.frck_export_path.clone(),
            })
        }

        pub async fn launch(&self) -> RtckResult<tokio::process::Child> {
            let mut c = tokio::process::Command::new(&self.bin);
            let mut c = c.arg("--api-sock").arg(&self.socket);
            match &self.config_path {
                Some(config_path) => c = c.arg("--config-file").arg(&config_path),
                None => (),
            }
            Ok(c.spawn()?)
        }

        /// Waiting for the socket set by firecracker
        #[cfg(feature = "tokio")]
        pub async fn waiting_socket(&self, timeout: tokio::time::Duration) -> RtckResult<()> {
            // FIXME: better error handling. Give it a class.
            Ok(tokio::time::timeout(timeout, async {
                while tokio::fs::try_exists(&self.socket).await.is_err() {}
            })
            .await
            .map_err(|_| {
                RtckError::new(
                    RtckErrorClass::RemoteError,
                    "Remote socket set up timeout".to_string(),
                )
            })?)
        }

        /// Connect to the socket
        #[cfg(feature = "tokio")]
        pub async fn connect(&self) -> RtckResult<tokio::io::BufStream<tokio::net::UnixStream>> {
            Ok(tokio::io::BufStream::new(
                tokio::net::UnixStream::connect(&self.socket).await?,
            ))
        }
    }
}

pub mod jailer {
    use std::{path::PathBuf, sync::Arc};

    use parking_lot::{Condvar, Mutex};
    use serde::{Deserialize, Serialize};

    use crate::{
        config::GlobalConfig, local::handle_entry_default, RtckError, RtckErrorClass, RtckResult,
    };

    use super::{handle_entry, handle_entry_ref};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Jailer {
        // Path to local jailer bin
        // Usually something like `/usr/bin/jailer`
        bin: String,

        // Id of this jailer
        id: String,

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

        // Desired path of the socket
        socket: Option<String>,

        // Path to the config file
        config_path: Option<String>,

        // Jailer workspace directory
        jailer_workspace_dir: Option<PathBuf>,

        // Socket path seen by Rtck
        socket_path_export: Option<PathBuf>,

        // Config file path seen by firecracker
        config_path_jailed: Option<PathBuf>,

        // Machine log path seen by firecracker
        machine_log_path_jailed: Option<String>,

        // Machine log path seen by Rtck
        machine_log_path_export: Option<PathBuf>,

        // Metrics path seen by firecracker
        metrics_path_jailed: Option<String>,

        // Metrics path seen by Rtck
        metrics_path_export: Option<PathBuf>,
    }

    impl Jailer {
        pub fn get_socket_path_exported(&self) -> Option<&PathBuf> {
            self.socket_path_export.as_ref()
        }

        pub fn get_log_path_exported(&self) -> Option<&PathBuf> {
            self.machine_log_path_export.as_ref()
        }

        pub fn get_metrics_path_exported(&self) -> Option<&PathBuf> {
            self.metrics_path_export.as_ref()
        }

        pub fn get_jailer_workspace_dir(&self) -> RtckResult<&PathBuf> {
            Ok(handle_entry_ref(self.jailer_workspace_dir.as_ref())?)
        }
    }

    impl Jailer {
        pub fn from_config(config: &GlobalConfig) -> RtckResult<Self> {
            let jailer_config = config.jailer_config.as_ref().ok_or(RtckError::new(
                RtckErrorClass::ConfigError,
                "Missing jailer config".to_string(),
            ))?;

            const DEFAULT_CHROOT_BASE_DIR: &'static str = "/srv/jailer";
            Ok(Self {
                bin: handle_entry(&jailer_config.jailer_bin)?,
                id: handle_entry(&jailer_config.id)?,
                exec_file: handle_entry(&jailer_config.exec_file)?,
                uid: handle_entry(&jailer_config.uid)?,
                gid: handle_entry(&jailer_config.gid)?,
                chroot_base_dir: handle_entry_default(
                    &jailer_config.chroot_base_dir,
                    DEFAULT_CHROOT_BASE_DIR.into(),
                ),
                daemonize: jailer_config.daemonize.unwrap_or(false),
                socket: config.socket_path.clone(),
                config_path: config.frck_export_path.clone(),

                jailer_workspace_dir: None,
                socket_path_export: None,
                config_path_jailed: None,
                machine_log_path_jailed: match &config.frck_config {
                    None => None,
                    Some(frck_config) => match &frck_config.logger {
                        None => None,
                        Some(logger) => Some(logger.log_path.clone()),
                    },
                },
                machine_log_path_export: None,
                metrics_path_jailed: match &config.frck_config {
                    None => None,
                    Some(frck_config) => match &frck_config.metrics {
                        None => None,
                        Some(metrics) => Some(metrics.metrics_path.clone()),
                    },
                },
                metrics_path_export: None,
            })
        }

        pub fn jail(&mut self) -> RtckResult<()> {
            use crate::local::possible_malformed_entry;
            let id = &self.id;

            let temp_binding = PathBuf::from(&self.exec_file);
            let exec_file_name = possible_malformed_entry(temp_binding.file_name())?;

            let chroot_base_dir = &self.chroot_base_dir;

            const ROOT_FOLDER_NAME: &'static str = "root";
            let jailer_workspace_dir = PathBuf::from(chroot_base_dir)
                .join(exec_file_name)
                .join(id)
                .join(ROOT_FOLDER_NAME);

            self.jailer_workspace_dir = Some(jailer_workspace_dir.clone());

            const DEFAULT_SOCKET_PATH_UNDER_JAILER: &'static str = "run/firecracker.socket";
            let socket_path =
                handle_entry_default(&self.socket, DEFAULT_SOCKET_PATH_UNDER_JAILER.to_string());
            self.socket_path_export = Some(jailer_workspace_dir.join(socket_path));

            match &self.config_path {
                None => (),
                Some(config_path) => {
                    // Copy the config file into the jailer
                    const DEFAULT_CONFIG_PATH_JAILED: &'static str = "config/config.json";
                    self.config_path_jailed = Some(DEFAULT_CONFIG_PATH_JAILED.into());

                    let config_path_export = jailer_workspace_dir.join(DEFAULT_CONFIG_PATH_JAILED);
                    std::fs::copy(config_path, config_path_export)?;
                }
            }

            match &self.machine_log_path_jailed {
                None => (),
                Some(log_path) => {
                    self.machine_log_path_export = Some(jailer_workspace_dir.join(&log_path));
                }
            }

            match &self.metrics_path_jailed {
                None => (),
                Some(metrics_path) => {
                    self.metrics_path_export = Some(jailer_workspace_dir.join(&metrics_path));
                }
            }

            Ok(())
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

            Ok(cmd.spawn()?)
        }

        /// Waiting for the socket set by firecracker
        pub fn waiting_socket(&self, timeout: std::time::Duration) -> RtckResult<()> {
            let pair = Arc::new((Mutex::new(false), Condvar::new()));
            let pair_peer = Arc::clone(&pair);

            // Wait for the socket
            let path = PathBuf::from(handle_entry_ref(self.socket.as_ref())?);
            std::thread::spawn(move || {
                let &(ref lock, ref cvar) = &*pair_peer;
                let mut created = lock.lock();

                while !path.exists() {}

                *created = true;
                cvar.notify_one();
            });

            let &(ref lock, ref cvar) = &*pair;
            let created = lock.lock();
            if !*created {
                let result = cvar.wait_for(&mut lock.lock(), timeout);
                if result.timed_out() {
                    return Err(RtckError::new(
                        RtckErrorClass::RemoteError,
                        "Remote socket set up timeout".to_string(),
                    ));
                }
            }

            Ok(())
        }

        /// Connect to the socket
        pub fn connect(
            &self,
        ) -> RtckResult<bufstream::BufStream<std::os::unix::net::UnixStream>> {
            Ok(bufstream::BufStream::new(
                std::os::unix::net::UnixStream::connect(handle_entry(&self.socket_path_export)?)?,
            ))
        }
    }
}

pub mod jailer_async {
    use std::path::PathBuf;

    use crate::{
        config::GlobalConfig,
        local::{handle_entry, handle_entry_default, handle_entry_ref},
        RtckError, RtckErrorClass, RtckResult,
    };

    pub struct JailerAsync {
        // Path to local jailer bin
        // Usually something like `/usr/bin/jailer`
        bin: String,

        // Id of this jailer
        id: String,

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

        // Desired path of the socket
        socket: Option<String>,

        // Path to the config file
        config_path: Option<String>,

        // Jailer workspace directory
        jailer_workspace_dir: Option<PathBuf>,

        // Socket path seen by Rtck
        socket_path_export: Option<PathBuf>,

        // Config file path seen by firecracker
        config_path_jailed: Option<PathBuf>,

        // Machine log path seen by firecracker
        machine_log_path_jailed: Option<String>,

        // Machine log path seen by Rtck
        machine_log_path_export: Option<PathBuf>,

        // Metrics path seen by firecracker
        metrics_path_jailed: Option<String>,

        // Metrics path seen by Rtck
        metrics_path_export: Option<PathBuf>,
    }

    impl JailerAsync {
        pub fn get_socket_path_exported(&self) -> Option<&PathBuf> {
            self.socket_path_export.as_ref()
        }

        pub fn get_log_path_exported(&self) -> Option<&PathBuf> {
            self.machine_log_path_export.as_ref()
        }

        pub fn get_metrics_path_exported(&self) -> Option<&PathBuf> {
            self.metrics_path_export.as_ref()
        }

        pub fn get_jailer_workspace_dir(&self) -> RtckResult<&PathBuf> {
            Ok(handle_entry_ref(self.jailer_workspace_dir.as_ref())?)
        }
    }

    impl JailerAsync {
        pub fn from_config(config: &GlobalConfig) -> RtckResult<Self> {
            config.validate()?;

            let jailer_config = config.jailer_config.as_ref().ok_or(RtckError::new(
                RtckErrorClass::ConfigError,
                "Missing jailer config".to_string(),
            ))?;

            const DEFAULT_CHROOT_BASE_DIR: &'static str = "/srv/jailer";
            Ok(Self {
                bin: handle_entry(&jailer_config.jailer_bin)?,
                id: handle_entry(&jailer_config.id)?,
                exec_file: handle_entry(&jailer_config.exec_file)?,
                uid: handle_entry(&jailer_config.uid)?,
                gid: handle_entry(&jailer_config.gid)?,
                chroot_base_dir: handle_entry_default(
                    &jailer_config.chroot_base_dir,
                    DEFAULT_CHROOT_BASE_DIR.into(),
                ),
                daemonize: jailer_config.daemonize.unwrap_or(false),
                socket: config.socket_path.clone(),
                config_path: config.frck_export_path.clone(),

                jailer_workspace_dir: None,
                socket_path_export: None,
                config_path_jailed: None,
                machine_log_path_jailed: match &config.frck_config {
                    None => None,
                    Some(frck_config) => match &frck_config.logger {
                        None => None,
                        Some(logger) => Some(logger.log_path.clone()),
                    },
                },
                machine_log_path_export: None,
                metrics_path_jailed: match &config.frck_config {
                    None => None,
                    Some(frck_config) => match &frck_config.metrics {
                        None => None,
                        Some(metrics) => Some(metrics.metrics_path.clone()),
                    },
                },
                metrics_path_export: None,
            })
        }

        pub fn jail(&mut self) -> RtckResult<()> {
            use crate::local::possible_malformed_entry;
            let id = &self.id;

            let temp_binding = PathBuf::from(&self.exec_file);
            let exec_file_name = possible_malformed_entry(temp_binding.file_name())?;

            let chroot_base_dir = &self.chroot_base_dir;

            const ROOT_FOLDER_NAME: &'static str = "root";
            let jailer_workspace_dir = PathBuf::from(chroot_base_dir)
                .join(exec_file_name)
                .join(id)
                .join(ROOT_FOLDER_NAME);

            self.jailer_workspace_dir = Some(jailer_workspace_dir.clone());

            const DEFAULT_SOCKET_PATH_UNDER_JAILER: &'static str = "run/firecracker.socket";
            let socket_path =
                handle_entry_default(&self.socket, DEFAULT_SOCKET_PATH_UNDER_JAILER.to_string());
            self.socket_path_export = Some(jailer_workspace_dir.join(socket_path));

            match &self.config_path {
                None => (),
                Some(config_path) => {
                    // Copy the config file into the jailer
                    const DEFAULT_CONFIG_PATH_JAILED: &'static str = "config/config.json";
                    self.config_path_jailed = Some(DEFAULT_CONFIG_PATH_JAILED.into());

                    let config_path_export = jailer_workspace_dir.join(DEFAULT_CONFIG_PATH_JAILED);
                    std::fs::copy(config_path, config_path_export)?;
                }
            }

            match &self.machine_log_path_jailed {
                None => (),
                Some(log_path) => {
                    self.machine_log_path_export = Some(jailer_workspace_dir.join(&log_path));
                }
            }

            match &self.metrics_path_jailed {
                None => (),
                Some(metrics_path) => {
                    self.metrics_path_export = Some(jailer_workspace_dir.join(&metrics_path));
                }
            }

            Ok(())
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

            Ok(cmd.spawn()?)
        }

        /// Waiting for the socket set by firecracker
        #[cfg(feature = "tokio")]
        pub async fn waiting_socket(&self, timeout: tokio::time::Duration) -> RtckResult<()> {
            let socket_path = handle_entry(&self.socket_path_export)?;
            let socket_path = socket_path.as_os_str();
            // FIXME: better error handling. Give it a class.
            Ok(tokio::time::timeout(timeout, async {
                while tokio::fs::try_exists(socket_path).await.is_err() {}
            })
            .await
            .map_err(|_| {
                RtckError::new(
                    RtckErrorClass::RemoteError,
                    "Remote socket set up timeout".to_string(),
                )
            })?)
        }

        /// Connect to the socket
        #[cfg(feature = "tokio")]
        pub async fn connect(&self) -> RtckResult<tokio::io::BufStream<tokio::net::UnixStream>> {
            Ok(tokio::io::BufStream::new(
                tokio::net::UnixStream::connect(handle_entry(&self.socket_path_export)?).await?,
            ))
        }
    }
}

pub mod local {
    use std::path::{Path, PathBuf};

    use crate::{config::GlobalConfig, RtckResult};

    use super::{firecracker::Firecracker, jailer::Jailer};

    pub struct Local {
        socket_path: PathBuf,
        machine_log_path: Option<PathBuf>,
        metrics_path: Option<PathBuf>,
        jail_path: Option<PathBuf>,

        machine_log_clear: Option<bool>,
        metrics_clear: Option<bool>,
        network_clear: Option<bool>,
    }

    impl Local {
        /// Construct a LocalAsync with information from JailerAsync and GlobalConfig
        pub fn from_jailer(jailer: &Jailer, config: &GlobalConfig) -> RtckResult<Self> {
            let socket_path = jailer.get_socket_path_exported().cloned();
            let socket_path = if let Some(socket_path) = socket_path {
                socket_path
            } else {
                log::error!("[LocalAsync::from_jailer fail to get socket_path]");
                return Err(crate::RtckError::new(
                    crate::RtckErrorClass::ConfigError,
                    "Fail to get socket_path".to_string(),
                ));
            };
            let machine_log_path = jailer.get_log_path_exported().cloned();
            let metrics_path = jailer.get_metrics_path_exported().cloned();

            // If construct from jailer, then the jailer path must be known
            let jail_path = jailer.get_jailer_workspace_dir()?.clone();

            Ok(Self {
                socket_path,
                machine_log_path,
                metrics_path,
                jail_path: Some(jail_path),
                machine_log_clear: config.log_clear,
                metrics_clear: config.metrics_clear,
                network_clear: config.network_clear,
            })
        }

        /// Construct a LocalAsync with information from FirecrackerAsync and GlobalConfig
        pub fn from_frck(frck: &Firecracker, config: &GlobalConfig) -> RtckResult<Self> {
            let socket_path = PathBuf::from(frck.get_socket().clone());
            let machine_log_path = match &config.frck_config {
                None => None,
                Some(frck_config) => match &frck_config.logger {
                    None => None,
                    Some(logger) => Some(logger.log_path.clone()),
                },
            }
            .map(PathBuf::from);
            let metrics_path = match &config.frck_config {
                None => None,
                Some(frck_config) => match &frck_config.metrics {
                    None => None,
                    Some(metrics) => Some(metrics.metrics_path.clone()),
                },
            }
            .map(PathBuf::from);
            let jail_path = None;

            Ok(Self {
                socket_path,
                machine_log_path,
                metrics_path,
                jail_path,
                machine_log_clear: config.log_clear,
                metrics_clear: config.metrics_clear,
                network_clear: config.network_clear,
            })
        }

        /// Setup basic environment
        pub fn setup(&self) -> RtckResult<()> {
            self.create_machine_log()?;
            self.create_jailer_dir()?;

            Ok(())
        }

        /// Create machine log
        pub fn create_machine_log(&self) -> RtckResult<()> {
            if let Some(path) = &self.machine_log_path {
                std::fs::File::create(path)?;
            }
            Ok(())
        }

        /// Create jailer directory
        pub fn create_jailer_dir(&self) -> RtckResult<()> {
            if let Some(path) = &self.jail_path {
                std::fs::create_dir_all(path)?;
            }
            Ok(())
        }

        /// Move the log to desired position
        /// Might need several switch from and to jail
        pub fn cp_machine_log<P: AsRef<Path>>(&self, to: P) -> RtckResult<()> {
            match &self.machine_log_path {
                None => log::error!("Fail to move machine log to {:?}", to.as_ref()),
                Some(path) => {
                    let res = std::fs::copy(path, to.as_ref());
                    if res.is_err() {
                        log::error!("Fail to move machine log to {:?}", to.as_ref());
                    }
                }
            }

            Ok(())
        }

        /// Do full cleaning up, ignoring possible failures and report them to logger
        pub fn full_clean(&self) {
            match self.rm_socket() {
                Ok(_) => (),
                Err(e) => log::error!("Fail to remove socket, {e}"),
            }

            match self.rm_machine_log() {
                Ok(_) => (),
                Err(e) => log::error!("Fail to remove machine log, {e}"),
            }

            match self.rm_jail() {
                Ok(_) => (),
                Err(e) => log::error!("Fail to remove jailer directory, {e}"),
            }
        }

        /// Remove only the socket
        pub fn rm_socket(&self) -> RtckResult<()> {
            Ok(std::fs::remove_file(&self.socket_path)?)
        }

        /// Remove the machine log
        pub fn rm_machine_log(&self) -> RtckResult<()> {
            if let Some(true) = self.machine_log_clear {
                if let Some(path) = &self.machine_log_path {
                    std::fs::remove_file(path)?
                }
            }
            Ok(())
        }

        /// Remove the metrics
        pub fn rm_metrics(&self) -> RtckResult<()> {
            if let Some(true) = self.metrics_clear {
                if let Some(path) = &self.metrics_path {
                    std::fs::remove_file(path)?;
                }
            }
            Ok(())
        }

        /// Remove the networks
        pub fn rm_networks(&self) -> RtckResult<()> {
            if let Some(true) = self.network_clear {
                todo!()
            }
            Ok(())
        }

        /// Remove the jail directory
        pub fn rm_jail(&self) -> RtckResult<()> {
            if let Some(path) = &self.jail_path {
                std::fs::remove_dir_all(path)?
            }
            Ok(())
        }
    }
}

#[cfg(feature = "tokio")]
pub mod local_async {
    use std::path::{Path, PathBuf};

    use crate::{config::GlobalConfig, RtckResult};

    use super::{firecracker_async::FirecrackerAsync, jailer_async::JailerAsync};

    pub struct LocalAsync {
        socket_path: PathBuf,
        machine_log_path: Option<PathBuf>,
        metrics_path: Option<PathBuf>,
        jail_path: Option<PathBuf>,

        machine_log_clear: Option<bool>,
        metrics_clear: Option<bool>,
        network_clear: Option<bool>,
    }

    impl LocalAsync {
        /// Construct a LocalAsync with information from JailerAsync and GlobalConfig
        pub fn from_jailer(jailer: &JailerAsync, config: &GlobalConfig) -> RtckResult<Self> {
            let socket_path = jailer.get_socket_path_exported().cloned();
            let socket_path = if let Some(socket_path) = socket_path {
                socket_path
            } else {
                log::error!("[LocalAsync::from_jailer fail to get socket_path]");
                return Err(crate::RtckError::new(
                    crate::RtckErrorClass::ConfigError,
                    "Fail to get socket_path".to_string(),
                ));
            };
            let machine_log_path = jailer.get_log_path_exported().cloned();
            let metrics_path = jailer.get_metrics_path_exported().cloned();

            // If construct from jailer, then the jailer path must be known
            let jail_path = jailer.get_jailer_workspace_dir()?.clone();

            Ok(Self {
                socket_path,
                machine_log_path,
                metrics_path,
                jail_path: Some(jail_path),
                machine_log_clear: config.log_clear,
                metrics_clear: config.metrics_clear,
                network_clear: config.network_clear,
            })
        }

        /// Construct a LocalAsync with information from FirecrackerAsync and GlobalConfig
        pub fn from_frck(frck: &FirecrackerAsync, config: &GlobalConfig) -> RtckResult<Self> {
            let socket_path = PathBuf::from(frck.get_socket().clone());
            let machine_log_path = match &config.frck_config {
                None => None,
                Some(frck_config) => match &frck_config.logger {
                    None => None,
                    Some(logger) => Some(logger.log_path.clone()),
                },
            }
            .map(PathBuf::from);
            let metrics_path = match &config.frck_config {
                None => None,
                Some(frck_config) => match &frck_config.metrics {
                    None => None,
                    Some(metrics) => Some(metrics.metrics_path.clone()),
                },
            }
            .map(PathBuf::from);
            let jail_path = None;

            Ok(Self {
                socket_path,
                machine_log_path,
                metrics_path,
                jail_path,
                machine_log_clear: config.log_clear,
                metrics_clear: config.metrics_clear,
                network_clear: config.network_clear,
            })
        }

        /// Setup basic environment
        pub async fn setup(&self) -> RtckResult<()> {
            self.create_machine_log().await?;
            self.create_jailer_dir().await?;

            Ok(())
        }

        /// Create machine log
        #[cfg(feature = "tokio")]
        pub async fn create_machine_log(&self) -> RtckResult<()> {
            if let Some(path) = &self.machine_log_path {
                tokio::fs::File::create(path).await?;
            }
            Ok(())
        }

        /// Create jailer directory
        #[cfg(feature = "tokio")]
        pub async fn create_jailer_dir(&self) -> RtckResult<()> {
            if let Some(path) = &self.jail_path {
                tokio::fs::create_dir_all(path).await?;
            }
            Ok(())
        }

        /// Move the log to desired position
        /// Might need several switch from and to jail
        #[cfg(feature = "tokio")]
        pub async fn cp_machine_log<P: AsRef<Path>>(&self, to: P) -> RtckResult<()> {
            match &self.machine_log_path {
                None => log::error!("Fail to move machine log to {:?}", to.as_ref()),
                Some(path) => {
                    let res = tokio::fs::copy(path, to.as_ref()).await;
                    if res.is_err() {
                        log::error!("Fail to move machine log to {:?}", to.as_ref());
                    }
                }
            }

            Ok(())
        }

        /// Do full cleaning up, ignoring possible failures and report them to logger
        pub async fn full_clean(&self) {
            match self.rm_socket().await {
                Ok(_) => (),
                Err(e) => log::error!("Fail to remove socket, {e}"),
            }

            match self.rm_machine_log().await {
                Ok(_) => (),
                Err(e) => log::error!("Fail to remove machine log, {e}"),
            }

            match self.rm_jail().await {
                Ok(_) => (),
                Err(e) => log::error!("Fail to remove jailer directory, {e}"),
            }
        }

        /// Remove the socket
        #[cfg(feature = "tokio")]
        pub async fn rm_socket(&self) -> RtckResult<()> {
            Ok(tokio::fs::remove_file(&self.socket_path).await?)
        }

        /// Remove the machine log
        #[cfg(feature = "tokio")]
        pub async fn rm_machine_log(&self) -> RtckResult<()> {
            if let Some(true) = self.machine_log_clear {
                if let Some(path) = &self.machine_log_path {
                    tokio::fs::remove_file(path).await?
                }
            }
            Ok(())
        }

        /// Remove the metrics
        #[cfg(feature = "tokio")]
        pub async fn rm_metrics(&self) -> RtckResult<()> {
            if let Some(true) = self.metrics_clear {
                if let Some(path) = &self.metrics_path {
                    tokio::fs::remove_file(path).await?
                }
            }
            Ok(())
        }

        /// Remove the networks
        #[cfg(feature = "tokio")]
        pub async fn rm_networks(&self) -> RtckResult<()> {
            if let Some(true) = self.network_clear {
                todo!()
            }
            Ok(())
        }

        /// Remove the jail directory
        #[cfg(feature = "tokio")]
        pub async fn rm_jail(&self) -> RtckResult<()> {
            if let Some(path) = &self.jail_path {
                tokio::fs::remove_dir_all(path).await?
            }
            Ok(())
        }
    }
}

#[doc(hidden)]
pub(crate) fn handle_entry<T: Clone>(option: &Option<T>) -> RtckResult<T> {
    option.clone().ok_or(RtckError::new(
        RtckErrorClass::ConfigError,
        "Missing config entry".to_string(),
    ))
}

#[doc(hidden)]
pub(crate) fn handle_entry_ref<T>(option: Option<&T>) -> RtckResult<&T> {
    option.ok_or(RtckError::new(
        RtckErrorClass::ConfigError,
        "Missing config entry".to_string(),
    ))
}

#[doc(hidden)]
pub(crate) fn handle_entry_default<T: Clone>(option: &Option<T>, default: T) -> T {
    option.clone().unwrap_or(default)
}

#[doc(hidden)]
pub(crate) fn possible_malformed_entry<T: ?Sized>(option: Option<&T>) -> RtckResult<&T> {
    option.ok_or(RtckError::new(
        RtckErrorClass::ConfigError,
        "Malformed config entry".to_string(),
    ))
}
