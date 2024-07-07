/// Module for manipulating host firecracker process
pub mod firecracker {

    use std::{
        os::unix::net::UnixStream,
        path::PathBuf,
        sync::{Arc, Condvar, Mutex},
    };

    use crate::{config::GlobalConfig, local::handle_entry, RtckError, RtckResult};

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
    use serde::{Deserialize, Serialize};
    use tokio::net::UnixStream;

    use crate::{config::GlobalConfig, local::handle_entry, RtckError, RtckResult};

    /// Unlike using jailer, when using bare firecracker, socket path and lock path must be specified
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FirecrackerAsync {
        // Path to local firecracker bin
        // Usually something like `/usr/bin/firecracker` if not using jailer
        bin: String,

        // Desired path of the socket
        socket: String,

        // Desired path of the lock
        lock_path: String,

        // Path to the config file
        config_path: Option<String>,
    }

    impl FirecrackerAsync {
        pub fn get_socket(&self) -> &String {
            &self.socket
        }

        pub fn get_lock_path(&self) -> &String {
            &self.lock_path
        }
    }

    impl FirecrackerAsync {
        pub fn from_config(config: &GlobalConfig) -> RtckResult<Self> {
            Ok(Self {
                bin: handle_entry(&config.frck_bin)?,
                socket: handle_entry(&config.socket_path)?,
                lock_path: handle_entry(&config.lock_path)?,
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
        pub async fn connect(&self) -> RtckResult<UnixStream> {
            UnixStream::connect(&self.socket)
                .await
                .map_err(|_| RtckError::Firecracker("connecting socket".to_string()))
        }
    }
}
