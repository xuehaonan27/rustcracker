pub mod local {
    use std::path::{Path, PathBuf};

    use crate::{
        config::GlobalConfig, firecracker::firecracker::Firecracker, jailer::jailer::Jailer,
        RtckResult,
    };

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

    use crate::{
        config::GlobalConfig, firecracker::firecracker_async::FirecrackerAsync,
        jailer::jailer_async::JailerAsync, RtckResult,
    };

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

use crate::{RtckError, RtckErrorClass, RtckResult};

#[doc(hidden)]
pub(crate) fn handle_entry<T: Clone>(option: &Option<T>) -> RtckResult<T> {
    option.clone().ok_or(RtckError::new(
        RtckErrorClass::ConfigError,
        "Missing config entry".to_string(),
    ))
}
