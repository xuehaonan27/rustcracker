/// Module for manipulating host firecracker process
pub mod firecracker {

    use std::{path::PathBuf, sync::Arc};

    use parking_lot::{Condvar, Mutex};

    use crate::{config::GlobalConfig, local::handle_entry, RtckError, RtckErrorClass, RtckResult};

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
    use crate::{config::GlobalConfig, local::handle_entry, RtckError, RtckErrorClass, RtckResult};

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
