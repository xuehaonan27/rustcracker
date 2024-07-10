use crate::{RtckError, RtckResult};

pub mod jailer {
    use std::{
        os::unix::net::UnixStream,
        path::PathBuf,
        sync::{Arc, Condvar, Mutex},
    };

    use serde::{Deserialize, Serialize};

    use crate::{
        config::HypervisorConfig, handle_entry, jailer::handle_entry_default, RtckError, RtckResult,
    };

    use super::handle_entry_ref;

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

        pub fn get_jailer_workspace_dir(&self) -> Option<&PathBuf> {
            self.jailer_workspace_dir.as_ref()
        }
    }

    impl Jailer {
        pub fn from_config(config: &HypervisorConfig) -> RtckResult<Self> {
            let jailer_config = config
                .jailer_config
                .as_ref()
                .ok_or(RtckError::Config("Missing jailer config".to_string()))?;

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
                jailer_workspace_dir: None,
                socket: config.socket_path.clone(),
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

        pub fn jail(&mut self) -> RtckResult<()> {
            let id = &self.id;

            let temp_binding = PathBuf::from(&self.exec_file);
            let exec_file_name = *handle_entry_ref(&temp_binding.file_name())?;

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

            const DEFAULT_LOCK_PATH_UNDER_JAILER: &'static str = "run/firecracker.lock";
            let lock_path =
                handle_entry_default(&self.lock_path, DEFAULT_LOCK_PATH_UNDER_JAILER.to_string());
            self.lock_path_export = Some(jailer_workspace_dir.join(lock_path));

            const DEFAULT_LOG_PATH_UNDER_JAILER: &'static str = "run/firecracker.log";
            let log_path =
                handle_entry_default(&self.log_path, DEFAULT_LOG_PATH_UNDER_JAILER.to_string());
            self.log_path_export = Some(jailer_workspace_dir.join(log_path));

            const DEFAULT_METRICS_PATH_UNDER_JAILER: &'static str = "run/firecracker.metrics";
            let metrics_path = handle_entry_default(
                &self.metrics_path,
                DEFAULT_METRICS_PATH_UNDER_JAILER.to_string(),
            );
            self.metrics_path_export = Some(jailer_workspace_dir.join(metrics_path));

            match &self.config_path {
                // not using config exported config, skipping
                None => (),
                Some(config_path) => {
                    // copy the config file into the jailer
                    const DEFAULT_CONFIG_PATH_JAILED: &'static str = "run/firecracker-config.json";
                    self.config_path_jailed = Some(DEFAULT_CONFIG_PATH_JAILED.into());

                    let config_path_export = jailer_workspace_dir.join(DEFAULT_CONFIG_PATH_JAILED);
                    std::fs::copy(config_path, config_path_export)
                        .map_err(|_| RtckError::FilesysIO("jailer copying config".to_string()))?;
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

            cmd.spawn()
                .map_err(|_| RtckError::Jailer("spawning jailer".to_string()))
        }

        /// Waiting for the socket set by firecracker
        pub fn waiting_socket(&self, timeout: std::time::Duration) -> RtckResult<()> {
            let pair = Arc::new((Mutex::new(false), Condvar::new()));
            let pair_peer = Arc::clone(&pair);

            // Wait for the socket
            let path = PathBuf::from(handle_entry_ref(&self.socket)?);
            std::thread::spawn(move || -> RtckResult<()> {
                let &(ref lock, ref cvar) = &*pair_peer;
                let mut created = lock
                    .lock()
                    .map_err(|_| RtckError::Jailer("waiting socket".to_string()))?;

                while !path.exists() {}

                *created = true;
                cvar.notify_one();

                Ok(())
            });

            let &(ref lock, ref cvar) = &*pair;
            let created = lock
                .lock()
                .map_err(|_| RtckError::Jailer("waiting socket".to_string()))?;
            if !*created {
                let result = cvar
                    .wait_timeout(
                        lock.lock()
                            .map_err(|_| RtckError::Jailer("waiting socket".to_string()))?,
                        timeout,
                    )
                    .map_err(|_| RtckError::Jailer("waiting socket".to_string()))?;
                if result.1.timed_out() {
                    return Err(RtckError::Jailer("remote socket timeout".to_string()));
                }
            }

            Ok(())
        }

        /// Connect to the socket
        pub fn connect(&self) -> RtckResult<UnixStream> {
            UnixStream::connect(handle_entry_ref(&self.socket_path_export)?)
                .map_err(|_| RtckError::Jailer("connecting socket".to_string()))
        }
    }
}

pub mod jailer_async {
    use std::{path::PathBuf, process::Stdio};

    use serde::{Deserialize, Serialize};
    use tokio::net::UnixStream;

    use crate::{
        config::HypervisorConfig, handle_entry, jailer::handle_entry_default, RtckError, RtckResult,
    };

    use super::handle_entry_ref;

    #[derive(Debug, Clone, Serialize, Deserialize)]
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

        // stdout redirection
        stdout_to: Option<String>,

        // stdout redirection exported
        stdout_to_exported: Option<PathBuf>,

        // stderr redirection
        stderr_to: Option<String>,

        // stderr redirection exported
        stderr_to_exported: Option<PathBuf>,
    }

    impl JailerAsync {
        pub fn get_uid(&self) -> u32 {
            self.uid
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

        pub fn get_stdout_redirection_exported(&self) -> Option<&PathBuf> {
            self.stdout_to_exported.as_ref()
        }

        pub fn get_stderr_redirection_exported(&self) -> Option<&PathBuf> {
            self.stderr_to_exported.as_ref()
        }
    }

    impl JailerAsync {
        pub fn from_config(config: &HypervisorConfig) -> RtckResult<Self> {
            config.validate()?;

            let jailer_config = config
                .jailer_config
                .as_ref()
                .ok_or(RtckError::Config("missing jailer config".to_string()))?;

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
                jailer_workspace_dir: None,
                socket: config.socket_path.clone(),
                socket_path_export: None,
                lock_path: config.lock_path.clone(),
                lock_path_export: None,
                config_path: config.frck_export_path.clone(),
                config_path_jailed: None,
                log_path: config.log_path.clone(),
                log_path_export: None,
                metrics_path: config.metrics_path.clone(),
                metrics_path_export: None,
                stdout_to: config.stdout_to.clone(),
                stdout_to_exported: None,
                stderr_to: config.stderr_to.clone(),
                stderr_to_exported: None,
            })
        }

        pub async fn jail(&mut self) -> RtckResult<()> {
            let id = &self.id;

            let temp_binding = PathBuf::from(&self.exec_file);
            let exec_file_name = *handle_entry_ref(&temp_binding.file_name())?;

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
            let socket_path = PathBuf::from(socket_path);
            let socket_path = if socket_path.is_absolute() {
                socket_path
                    .strip_prefix("/")
                    .map_err(|_| RtckError::Jailer("fail to strip absolute prefix".to_string()))?
            } else {
                socket_path.as_path()
            };
            self.socket_path_export = Some(jailer_workspace_dir.join(socket_path));

            const DEFAULT_LOCK_PATH_UNDER_JAILER: &'static str = "run/firecracker.lock";
            let lock_path =
                handle_entry_default(&self.lock_path, DEFAULT_LOCK_PATH_UNDER_JAILER.to_string());
            let lock_path = PathBuf::from(lock_path);
            let lock_path = if lock_path.is_absolute() {
                lock_path
                    .strip_prefix("/")
                    .map_err(|_| RtckError::Jailer("fail to strip absolute prefix".to_string()))?
            } else {
                lock_path.as_path()
            };
            self.lock_path_export = Some(jailer_workspace_dir.join(lock_path));

            const DEFAULT_LOG_PATH_UNDER_JAILER: &'static str = "run/firecracker.log";
            let log_path =
                handle_entry_default(&self.log_path, DEFAULT_LOG_PATH_UNDER_JAILER.to_string());
            let log_path = PathBuf::from(log_path);
            let log_path = if log_path.is_absolute() {
                log_path
                    .strip_prefix("/")
                    .map_err(|_| RtckError::Jailer("fail to strip absolute prefix".to_string()))?
            } else {
                log_path.as_path()
            };
            self.log_path_export = Some(jailer_workspace_dir.join(log_path));

            const DEFAULT_METRICS_PATH_UNDER_JAILER: &'static str = "run/firecracker.metrics";
            let metrics_path = handle_entry_default(
                &self.metrics_path,
                DEFAULT_METRICS_PATH_UNDER_JAILER.to_string(),
            );
            let metrics_path = PathBuf::from(metrics_path);
            let metrics_path = if metrics_path.is_absolute() {
                metrics_path
                    .strip_prefix("/")
                    .map_err(|_| RtckError::Jailer("fail to strip absolute prefix".to_string()))?
            } else {
                metrics_path.as_path()
            };
            self.metrics_path_export = Some(jailer_workspace_dir.join(metrics_path));

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
                        .map_err(|_| RtckError::FilesysIO("jailer copying config".to_string()))?;
                }
            }

            match &self.stdout_to {
                // not using stdout redirection, skipping
                None => (),
                Some(stdout_to) => {
                    let stdout_to = PathBuf::from(stdout_to);
                    let stdout_to = if stdout_to.is_absolute() {
                        stdout_to.strip_prefix("/").map_err(|_| {
                            RtckError::Jailer("fail to strip absolute prefix".to_string())
                        })?
                    } else {
                        stdout_to.as_path()
                    };
                    self.stdout_to_exported = Some(jailer_workspace_dir.join(stdout_to));
                }
            }

            match &self.stderr_to {
                None => (),
                Some(stderr_to) => {
                    let stderr_to = PathBuf::from(stderr_to);
                    let stderr_to = if stderr_to.is_absolute() {
                        stderr_to.strip_prefix("/").map_err(|_| {
                            RtckError::Jailer("fail to strip absolute prefix".to_string())
                        })?
                    } else {
                        stderr_to.as_path()
                    };
                    self.stderr_to_exported = Some(jailer_workspace_dir.join(stderr_to));
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

            match &self.stdout_to {
                Some(stdout_to) => {
                    let stdout = std::fs::File::open(stdout_to).map_err(|_| {
                        RtckError::FilesysIO("fail to open stdout redirection file".to_string())
                    })?;
                    cmd.stdout(Stdio::from(stdout));
                }
                None => (),
            }

            match &self.stderr_to {
                Some(stderr_to) => {
                    let stderr = std::fs::File::open(stderr_to).map_err(|_| {
                        RtckError::FilesysIO("fail to open stderr redirection file".to_string())
                    })?;
                    cmd.stdout(Stdio::from(stderr));
                }
                None => (),
            }

            cmd.spawn()
                .map_err(|_| RtckError::Jailer("spawning jailer".to_string()))
        }

        /// Waiting for the socket set by firecracker
        pub async fn waiting_socket(&self, timeout: tokio::time::Duration) -> RtckResult<()> {
            let socket_path = handle_entry(&self.socket_path_export)?;
            let socket_path = socket_path.as_os_str();
            // FIXME: better error handling. Give it a class.
            Ok(tokio::time::timeout(timeout, async {
                while tokio::fs::try_exists(socket_path).await.is_err() {}
            })
            .await
            .map_err(|_| RtckError::Jailer("remote socket timeout".to_string()))?)
        }

        /// Connect to the socket
        pub async fn connect(&self, retry: usize) -> RtckResult<UnixStream> {
            let mut trying = retry;
            let stream = loop {
                if trying == 0 {
                    return Err(RtckError::Firecracker(format!(
                        "fail to connect unix socket after {retry} tries"
                    )));
                }
                match UnixStream::connect(handle_entry_ref(&self.socket_path_export)?).await {
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

fn handle_entry_default<T: Clone>(entry: &Option<T>, default: T) -> T {
    if entry.as_ref().is_some() {
        entry.as_ref().unwrap().clone()
    } else {
        default
    }
}

fn handle_entry_ref<T>(entry: &Option<T>) -> RtckResult<&T> {
    entry
        .as_ref()
        .ok_or(RtckError::Jailer("missing entry".to_string()))
}

pub use jailer::Jailer;
pub use jailer_async::JailerAsync;
