
pub mod jailer {
    use std::{path::PathBuf, sync::Arc};

    use parking_lot::{Condvar, Mutex};
    use serde::{Deserialize, Serialize};

    use crate::{
        config::GlobalConfig, handle_entry_default, handle_entry_ref, local::handle_entry, RtckError, RtckErrorClass, RtckResult
    };

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
            let id = &self.id;

            use crate::possible_malformed_entry;
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
        config::GlobalConfig, handle_entry_default, handle_entry_ref, local::handle_entry, RtckError, RtckErrorClass, RtckResult
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
            let id = &self.id;

            use crate::possible_malformed_entry;
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