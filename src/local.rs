use crate::{RtckError, RtckErrorClass, RtckResult};

/// Module for manipulating host firecracker process
pub mod firecracker {
    use serde::{Deserialize, Serialize};

    use crate::{config::GlobalConfig, RtckError, RtckErrorClass, RtckResult};

    use super::handle_entry;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Firecracker {
        // Path to local firecracker bin
        // Usually something like `/usr/bin/firecracker` if not using jailer
        bin: String,

        // Desired path of the socket
        socket: String,

        // Path to the config file
        config: Option<String>,
    }

    impl Firecracker {
        pub fn from_config(config: &GlobalConfig) -> RtckResult<Self> {
            config.validate()?;

            Ok(Self {
                bin: config.frck_bin.clone(),
                socket: config.socket_path.clone(),
                config: config.frck_export.clone(),
            })
        }

        pub fn launch(&self) -> RtckResult<std::process::Child> {
            let mut c = std::process::Command::new(&self.bin);
            let mut c = c.arg("--api-sock").arg(&self.socket);
            match &self.config {
                Some(config) => c = c.arg("--config-file").arg(&config),
                None => (),
            }
            Ok(c.spawn()?)
        }

        #[cfg(feature = "tokio")]
        pub async fn launch_async(&self) -> RtckResult<tokio::process::Child> {
            let mut c = tokio::process::Command::new(&self.bin);
            let mut c = c.arg("--api-sock").arg(&self.socket);
            match &self.config {
                Some(config) => c = c.arg("--config-file").arg(&config),
                None => (),
            }
            Ok(c.spawn()?)
        }
    }

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
    }

    impl Jailer {
        pub fn from_config(config: &GlobalConfig) -> RtckResult<Self> {
            config.validate()?;

            let jailer_config = config.jailer_config.as_ref().ok_or(RtckError::new(
                RtckErrorClass::ConfigError,
                "Missing jailer config".to_string(),
            ))?;

            Ok(Self {
                bin: handle_entry(&jailer_config.jailer_bin)?,
                id: handle_entry(&jailer_config.id)?,
                exec_file: handle_entry(&jailer_config.exec_file)?,
                uid: handle_entry(&jailer_config.uid)?,
                gid: handle_entry(&jailer_config.gid)?,
            })
        }
    }
}

pub mod local {
    use std::{
        cell::Cell,
        fs::{create_dir_all, remove_dir_all, remove_file, File},
        path::Path,
    };

    use crate::{config::GlobalConfig, RtckError, RtckErrorClass, RtckResult};

    use super::firecracker::Firecracker;

    pub struct Local {
        // rtck_logger: L,
        frck: Firecracker,
        child: Cell<Option<std::process::Child>>,

        socket_path: String,
        machine_log_path: Option<String>,
        jail_path: Option<String>,
    }

    impl Local {
        /// Derive from config
        pub fn from_config(config: &GlobalConfig) -> RtckResult<Self> {
            let frck = Firecracker::from_config(config)?;
            let socket_path = config.socket_path.clone();
            let machine_log_path = config.frck_config.log_path.clone();
            let jail_path = match &config.jailer_config {
                None => None,
                Some(config) => config.chroot_base_dir.clone(),
            };

            Ok(Self {
                frck,
                child: Cell::new(None),
                socket_path,
                machine_log_path,
                jail_path,
            })
        }

        /// Launch the firecracker process
        /// Err if the process had been launched before
        pub fn start_firecracker(&self) -> RtckResult<()> {
            let child = self.child.take();
            if child.is_some() {
                self.child.set(child);
                return Err(RtckError::new(
                    RtckErrorClass::RemoteError,
                    "Firecracker already launched".to_string(),
                ));
            } else {
                self.child.set(Some(self.frck.launch()?));
                Ok(())
            }
        }

        /// Create machine log
        pub fn create_machine_log(&self) -> RtckResult<()> {
            if let Some(path) = &self.machine_log_path {
                File::create(path)?;
            }
            Ok(())
        }

        /// Create jailer directory
        pub fn create_jailer_dir(&self) -> RtckResult<()> {
            if let Some(path) = &self.jail_path {
                create_dir_all(path)?;
            }
            Ok(())
        }

        /// Switch the current working directory into the jail
        pub fn switch_to_jail(&self) -> RtckResult<()> {
            todo!()
        }

        /// Move the log to desired position
        /// Might need several switch from and to jail
        pub fn mv_machine_log<P: AsRef<Path>>(&self, to: P) -> RtckResult<()> {
            todo!()
        }

        /// Move the metrics to desired position
        /// Might need several switch from and to jail
        pub fn mv_metrics<P: AsRef<Path>>(&self, to: P) -> RtckResult<()> {
            todo!()
        }

        /// Do full cleaning up, ignoring possible failures and report them to logger
        pub fn full_clean(&self) {
            todo!()
        }

        /// Remove only the socket
        pub fn rm_socket(&self) -> RtckResult<()> {
            Ok(remove_file(&self.socket_path)?)
        }

        /// Remove the machine log
        pub fn rm_machine_log(&self) -> RtckResult<()> {
            if let Some(path) = &self.machine_log_path {
                remove_file(path)?
            }
            Ok(())
        }

        /// Remove the jail directory
        pub fn rm_jail(&self) -> RtckResult<()> {
            if let Some(path) = &self.jail_path {
                remove_dir_all(path)?
            }
            Ok(())
        }
    }
}

#[cfg(feature = "tokio")]
pub mod local_async {
    pub struct LocalAsync {}
}

#[doc(hidden)]
fn handle_entry<T: Clone>(option: &Option<T>) -> RtckResult<T> {
    option.clone().ok_or(RtckError::new(
        RtckErrorClass::ConfigError,
        "Missing jailer config entry".to_string(),
    ))
}
