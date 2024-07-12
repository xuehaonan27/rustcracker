/// Module for manipulating host firecracker process
pub mod firecracker {

    use std::{
        os::unix::net::UnixStream,
        path::PathBuf,
        sync::{Arc, Condvar, Mutex},
    };

    use crate::{config::HypervisorConfig, handle_entry, RtckError, RtckResult};

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
        pub fn from_config(config: &HypervisorConfig) -> RtckResult<Self> {
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
            c.spawn()
                .map_err(|_| RtckError::Firecracker("spawn fail".to_string()))
        }

        /// Waiting for the socket set by firecracker
        pub fn waiting_socket(&self, timeout: std::time::Duration) -> RtckResult<()> {
            let pair = Arc::new((Mutex::new(false), Condvar::new()));
            let pair_peer = Arc::clone(&pair);

            // Wait for the socket
            let path = PathBuf::from(&self.socket);
            std::thread::spawn(move || -> RtckResult<()> {
                let &(ref lock, ref cvar) = &*pair_peer;
                let mut created = lock
                    .lock()
                    .map_err(|_| RtckError::Firecracker("waiting socket".to_string()))?;

                while !path.exists() {}

                *created = true;
                cvar.notify_one();

                Ok(())
            });

            let &(ref lock, ref cvar) = &*pair;
            let created = lock
                .lock()
                .map_err(|_| RtckError::Firecracker("waiting socket".to_string()))?;

            if !*created {
                let result = cvar
                    .wait_timeout(
                        lock.lock()
                            .map_err(|_| RtckError::Firecracker("waiting socket".to_string()))?,
                        timeout,
                    )
                    .unwrap();
                if result.1.timed_out() {
                    // if result.timed_out() {
                    return Err(RtckError::Firecracker("remote socket timeout".to_string()));
                }
            }

            Ok(())
        }

        /// Connect to the socket
        pub fn connect(&self) -> RtckResult<UnixStream> {
            UnixStream::connect(&self.socket)
                .map_err(|_| RtckError::Firecracker("connecting socket".to_string()))
        }
    }
}

pub mod firecracker_async {
    use std::{path::PathBuf, process::Stdio};

    use serde::{Deserialize, Serialize};
    use tokio::net::UnixStream;

    use crate::{
        config::HypervisorConfig, handle_entry, jailer::JailerAsync, RtckError, RtckResult,
    };

    /// Unlike using jailer, when using bare firecracker, socket path and lock path must be specified
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FirecrackerAsync {
        pub(crate) id: String,

        // Path to local firecracker bin
        // Usually something like `/usr/bin/firecracker` if not using jailer
        pub(crate) bin: String,

        pub(crate) socket: PathBuf,

        pub(crate) lock_path: PathBuf,

        pub(crate) log_path: PathBuf,

        pub(crate) metrics_path: PathBuf,

        // Path to the config file
        pub(crate) config_path: Option<String>,

        // stdout redirection
        pub(crate) stdout_to: Option<PathBuf>,

        // stderr redirection
        pub(crate) stderr_to: Option<PathBuf>,
    }

    impl FirecrackerAsync {
        /// Using bare firecracker
        pub fn from_config(config: &HypervisorConfig) -> RtckResult<Self> {
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
                bin: handle_entry(&config.frck_bin)?,
                socket,
                lock_path: handle_entry(&config.lock_path)?.into(),
                log_path: handle_entry(&config.log_path)?.into(),
                metrics_path: handle_entry(&config.metrics_path)?.into(),
                config_path: config.frck_export_path.clone().and_then(|s| Some(s.into())),
                stdout_to: config
                    .stdout_to
                    .clone()
                    .and_then(|s| Some(PathBuf::from(s))),
                stderr_to: config
                    .stderr_to
                    .clone()
                    .and_then(|s| Some(PathBuf::from(s))),
            })
        }

        /// Using firecracker with jailer
        pub fn from_jailer(jailer: JailerAsync) -> RtckResult<Self> {
            let bin = jailer.get_firecracker_exec_file().clone();

            let socket = jailer
                .get_socket_path_exported()
                .ok_or(RtckError::Config(
                    "jailer without socket path exported".to_string(),
                ))?
                .clone();

            let lock_path = jailer
                .get_lock_path_exported()
                .ok_or(RtckError::Config(
                    "jailer without lock path exported".to_string(),
                ))?
                .clone();

            let log_path = jailer
                .get_log_path_exported()
                .ok_or(RtckError::Config(
                    "jailer without log path exported".to_string(),
                ))?
                .clone();

            let metrics_path = jailer
                .get_metrics_path_exported()
                .ok_or(RtckError::Config(
                    "jailer without metrics path exported".to_string(),
                ))?
                .clone();

            let config_path = jailer.get_config_path_exported().cloned();

            let stdout_to = jailer.get_stdout_redirection_exported().cloned();

            let stderr_to = jailer.get_stderr_redirection_exported().cloned();

            Ok(Self {
                id: jailer.id,
                bin,
                socket,
                lock_path,
                log_path,
                metrics_path,
                config_path,
                stdout_to,
                stderr_to,
            })
        }

        pub async fn launch(&self) -> RtckResult<tokio::process::Child> {
            let mut c = tokio::process::Command::new(&self.bin);
            let mut c = c.arg("--api-sock").arg(&self.socket);
            match &self.config_path {
                Some(config_path) => c = c.arg("--config-file").arg(&config_path),
                None => (),
            }

            match &self.stdout_to {
                Some(stdout_to) => {
                    let stdout = std::fs::File::open(stdout_to).map_err(|_| {
                        RtckError::FilesysIO("fail to open stdout redirection file".to_string())
                    })?;
                    c.stdout(Stdio::from(stdout));
                }
                None => (),
            }

            match &self.stderr_to {
                Some(stderr_to) => {
                    let stderr = std::fs::File::open(stderr_to).map_err(|_| {
                        RtckError::FilesysIO("fail to open stderr redirection file".to_string())
                    })?;
                    c.stdout(Stdio::from(stderr));
                }
                None => (),
            }

            c.spawn()
                .map_err(|_| RtckError::Firecracker("spawn fail".to_string()))
        }

        /// Waiting for the socket set by firecracker
        pub async fn waiting_socket(&self, timeout: tokio::time::Duration) -> RtckResult<()> {
            // FIXME: better error handling. Give it a class.
            Ok(tokio::time::timeout(timeout, async {
                while tokio::fs::try_exists(&self.socket).await.is_err() {}
            })
            .await
            .map_err(|_| RtckError::Config("remote socket timeout".to_string()))?)
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
                match UnixStream::connect(&self.socket).await {
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

        /// Get socket path
        pub fn get_socket_path(&self) -> PathBuf {
            self.socket.clone()
        }
    }
}

pub use firecracker::Firecracker;
pub use firecracker_async::FirecrackerAsync;
