pub mod machine {
    use std::io::{BufRead, Write};

    use crate::{
        config::GlobalConfig,
        events::events::{self, Event},
        firecracker::firecracker::Firecracker,
        jailer::jailer::Jailer,
        local::local::Local,
        models::{instance_action_info, snapshot_create_params, vm},
        rtck::Rtck,
        RtckError, RtckErrorClass, RtckResult,
    };

    pub struct Machine<S> {
        rtck: Rtck<S>,
        local: Local,
        jailer: Option<Jailer>,
        frck: Firecracker,
        config: GlobalConfig,
        child: std::process::Child,
    }

    impl<S> Machine<S> {
        /// Dump the global configuration of the machine for future use
        pub fn get_config(&self) -> GlobalConfig {
            self.config.clone()
        }
    }

    impl Machine<bufstream::BufStream<std::os::unix::net::UnixStream>> {
        /// Create a machine from scratch, using default stream
        pub fn create(config: &GlobalConfig) -> RtckResult<Self> {
            config.validate()?;

            let frck = Firecracker::from_config(config)?;
            let mut jailer = Jailer::from_config(config).ok();

            if config.frck_export_path.is_some() {
                config.export_config()?;
            }

            let (stream, child, local) = if config.using_jailer.unwrap() {
                // Set up for jailing
                assert!(jailer.is_some());
                let jailer = jailer.as_mut().unwrap();
                jailer.jail()?;
                let child = jailer.launch()?;
                jailer.waiting_socket(std::time::Duration::from_secs(3))?;
                (
                    jailer.connect()?,
                    child,
                    Local::from_jailer(jailer, config)?,
                )
            } else {
                // Firecracker launch and connect
                let child = frck.launch()?;
                frck.waiting_socket(std::time::Duration::from_secs(3))?;
                (frck.connect()?, child, Local::from_frck(&frck, config)?)
            };

            // Set up local environment
            local.full_clean();
            local.setup()?;

            let rtck = Rtck::from_stream(stream);

            Ok(Self {
                rtck,
                local,
                jailer,
                frck,
                config: config.clone(),
                child,
            })
        }
    }

    impl<S: BufRead + Write> Machine<S> {
        /// Ping firecracker to check its soundness
        pub fn pint_remote(&mut self) -> RtckResult<()> {
            let mut get_firecracker_version = events::GetFirecrackerVersion::new();
            Ok(self
                .rtck
                .execute(&mut get_firecracker_version)
                .map_err(|e| {
                    RtckError::new(
                        RtckErrorClass::RemoteError,
                        format!("Fail to ping remote {}", e.to_string()),
                    )
                })?)
        }

        /// Automatically configure the machine.
        /// User must guarantee that `config` passed to the machine contains
        /// valid firecracker configuration (`frck_config`).
        pub fn configure(&mut self) -> RtckResult<()> {
            if self.config.frck_export_path.is_some() {
                return Ok(());
            }

            use events::*;

            // User must guarantee that proper
            let frck_config = self.config.frck_config.as_ref().ok_or(RtckError::new(
                RtckErrorClass::ConfigError,
                "No proper firecracker configuration passed".to_string(),
            ))?;

            // Logger
            {
                if let Some(logger) = &frck_config.logger {
                    let mut put_logger = PutLogger::new(logger.clone());
                    self.rtck.execute(&mut put_logger)?;
                    if put_logger.is_err() {
                        log::error!(
                            "[PutLogger failed, error = {}]",
                            put_logger.get_res_mut().err()
                        );
                    }
                }
            }

            // Metrics
            {
                if let Some(metrics) = &frck_config.metrics {
                    let mut put_metrics = PutMetrics::new(metrics.clone());
                    self.rtck.execute(&mut put_metrics)?;
                    if put_metrics.is_err() {
                        log::error!(
                            "[PutMetrics failed, error = {}]",
                            put_metrics.get_res_mut().err()
                        );
                    }
                }
            }

            // Guest boot source
            {
                if let Some(boot_source) = &frck_config.boot_source {
                    let mut put_guest_boot_source = PutGuestBootSource::new(boot_source.clone());
                    self.rtck.execute(&mut put_guest_boot_source)?;
                    if put_guest_boot_source.is_err() {
                        log::error!(
                            "[PutGuestBootSource failed, error = {}]",
                            put_guest_boot_source.get_res_mut().err()
                        );
                    }
                }
            }

            // Guest drives
            {
                if let Some(drives) = &frck_config.drives {
                    for drive in drives {
                        let mut put_guest_drive_by_id = PutGuestDriveById::new(drive.clone());
                        self.rtck.execute(&mut put_guest_drive_by_id)?;
                        if put_guest_drive_by_id.is_err() {
                            log::error!(
                                "[PutGuestDriveById failed, error = {}]",
                                put_guest_drive_by_id.get_res_mut().err()
                            );
                        }
                    }
                }
            }

            // Guest network interfaces
            {
                if let Some(ifaces) = &frck_config.network_interfaces {
                    for iface in ifaces {
                        let mut put_guest_network_interface_by_id =
                            PutGuestNetworkInterfaceById::new(iface.clone());
                        self.rtck.execute(&mut put_guest_network_interface_by_id)?;
                        if put_guest_network_interface_by_id.is_err() {
                            log::error!(
                                "[PutGuestNetworkInterfaceById failed, error = {}]",
                                put_guest_network_interface_by_id.get_res_mut().err()
                            );
                        }
                    }
                }
            }

            // Vsocks
            {
                if let Some(vsocks) = &frck_config.vsock_devices {
                    for vsock in vsocks {
                        let mut put_guest_vsock = PutGuestVsock::new(vsock.clone());
                        self.rtck.execute(&mut put_guest_vsock)?;
                        if put_guest_vsock.is_err() {
                            log::error!(
                                "[PutGuestVsock failed, error = {}]",
                                put_guest_vsock.get_res_mut().err()
                            );
                        }
                    }
                }
            }

            // CPU configuration
            {
                if let Some(cpu_config) = &frck_config.cpu_config {
                    let mut put_cpu_configuration = PutCpuConfiguration::new(cpu_config.clone());
                    self.rtck.execute(&mut put_cpu_configuration)?;
                    if put_cpu_configuration.is_err() {
                        log::error!(
                            "[PutCpuConfiguration failed, error = {}]",
                            put_cpu_configuration.get_res_mut().err()
                        );
                    }
                }
            }

            // Machine configuration
            {
                if let Some(machine_config) = &frck_config.machine_config {
                    let mut put_machine_configuration =
                        PutMachineConfiguration::new(machine_config.clone());
                    self.rtck.execute(&mut put_machine_configuration)?;
                    if put_machine_configuration.is_err() {
                        log::error!(
                            "[PutMachineConfiguration failed, error = {}]",
                            put_machine_configuration.get_res_mut().err()
                        );
                    }
                }
            }

            // Balloon
            {
                if let Some(balloon) = &frck_config.balloon {
                    let mut put_balloon = PutBalloon::new(balloon.clone());
                    self.rtck.execute(&mut put_balloon)?;
                    if put_balloon.is_err() {
                        log::error!(
                            "[PutBalloon failed, error = {}]",
                            put_balloon.get_res_mut().err()
                        );
                    }
                }
            }

            // Entropy device
            {
                if let Some(entropy_device) = &frck_config.entropy_device {
                    let mut put_entropy = PutEntropy::new(entropy_device.clone());
                    self.rtck.execute(&mut put_entropy)?;
                    if put_entropy.is_err() {
                        log::error!(
                            "[PutEntropy failed, error = {}]",
                            put_entropy.get_res_mut().err()
                        );
                    }
                }
            }

            // Initial mmds content
            {
                if let Some(content) = &frck_config.init_metadata {
                    let mut put_mmds = PutMmds::new(content.clone());
                    self.rtck.execute(&mut put_mmds)?;
                    if put_mmds.is_err() {
                        log::error!("[PutMmds failed, error = {}]", put_mmds.get_res_mut().err());
                    }
                }
            }

            Ok(())
        }

        /// Start the machine by notifying the hypervisor
        pub fn start(&mut self) -> RtckResult<()> {
            let mut start_machine =
                events::CreateSyncAction::new(instance_action_info::InstanceActionInfo {
                    action_type: instance_action_info::ActionType::InstanceStart,
                });

            self.rtck.execute(&mut start_machine)?;
            Ok(())
        }

        /// Pause the machine by notifying the hypervisor
        pub fn pause(&mut self) -> RtckResult<()> {
            let mut pause_machine = events::PatchVm::new(vm::Vm {
                state: vm::State::Paused,
            });

            self.rtck.execute(&mut pause_machine)?;
            Ok(())
        }

        /// Resume the machine by notifying the hypervisor
        pub fn resume(&mut self) -> RtckResult<()> {
            let mut resume_machine = events::PatchVm::new(vm::Vm {
                state: vm::State::Resumed,
            });

            self.rtck.execute(&mut resume_machine)?;
            Ok(())
        }

        /// Stop the machine by notifying the hypervisor
        pub fn stop(&mut self) -> RtckResult<()> {
            let mut stop_machine =
                events::CreateSyncAction::new(instance_action_info::InstanceActionInfo {
                    action_type: instance_action_info::ActionType::SendCtrlAtlDel,
                });

            self.rtck.execute(&mut stop_machine)?;
            Ok(())
        }

        /// Stop the machine forcefully by killing the firecracker process
        pub fn stop_force(&mut self) -> RtckResult<()> {
            self.child.kill().map_err(|e| {
                log::error!("[Machine::stop_force killing failed, error = {}]", e);
                RtckError::new(
                    RtckErrorClass::MachineError,
                    "Fail to kill the machine".to_string(),
                )
            })
        }

        pub fn delete(&mut self) -> RtckResult<()> {
            // Stop the machine first
            self.stop()?;
            let mut query_status = events::DescribeInstance::new();
            self.rtck.execute(&mut query_status)?;

            if query_status.is_err() {
                log::error!(
                    "[Machine::delete query status failed, error = {}]",
                    query_status.get_res_mut().err()
                );
                return Err(RtckError::new(
                    RtckErrorClass::MachineError,
                    "Fail to query status".to_string(),
                ));
            }

            let state = query_status.get_res_mut().succ().state;

            use crate::models::instance_info;
            if state == instance_info::State::Running {
                log::warn!("[Machine::delete cannot stop the machine, killing...]");
                self.stop_force()?;
            }

            Ok(())
        }

        /// Delete the machine and do cleaning at the same time
        pub fn delete_and_clean(&mut self) -> RtckResult<()> {
            self.delete()?;
            self.local.full_clean();
            Ok(())
        }

        /// Create a snapshot
        pub fn snapshot<P: AsRef<str>, Q: AsRef<str>>(
            &mut self,
            state_path: P,
            mem_path: Q,
            _type: snapshot_create_params::SnapshotType,
        ) -> RtckResult<()> {
            let mut create_snapshot =
                events::CreateSnapshot::new(snapshot_create_params::SnapshotCreateParams {
                    mem_file_path: state_path.as_ref().to_string(),
                    snapshot_path: mem_path.as_ref().to_string(),
                    snapshot_type: Some(_type),
                    version: None,
                });

            self.rtck.execute(&mut create_snapshot)?;
            Ok(())
        }
    }
}

pub mod machine_async {
    use parking_lot::Mutex;

    use crate::{
        config::GlobalConfig,
        events::events_async::{self, EventAsync},
        firecracker::firecracker_async::FirecrackerAsync,
        jailer::jailer_async::JailerAsync,
        local::local_async::LocalAsync,
        models::{
            instance_action_info::{ActionType, InstanceActionInfo},
            snapshot_create_params::{SnapshotCreateParams, SnapshotType},
            vm,
        },
        rtck_async::RtckAsync,
        RtckError, RtckErrorClass, RtckResult,
    };

    pub struct Machine<S> {
        rtck: Mutex<RtckAsync<S>>,
        local: LocalAsync,
        jailer: Option<JailerAsync>,
        frck: FirecrackerAsync,
        config: GlobalConfig,
        child: Mutex<tokio::process::Child>,
    }

    impl<S> Machine<S> {
        /// Dump the global configuration of the machine for future use
        pub fn get_config(&self) -> GlobalConfig {
            self.config.clone()
        }
    }

    #[cfg(feature = "tokio")]
    impl Machine<tokio::io::BufStream<tokio::net::UnixStream>> {
        /// Create a machine from scratch, using default stream
        pub async fn create(config: &GlobalConfig) -> RtckResult<Self> {
            config.validate()?;

            let frck = FirecrackerAsync::from_config(config)?;
            let mut jailer = JailerAsync::from_config(config).ok();

            if config.frck_export_path.is_some() {
                config.export_config_async().await?;
            }

            let (stream, child, local) = if config.using_jailer.unwrap() {
                // Set up for jailing
                assert!(jailer.is_some());
                let jailer = jailer.as_mut().unwrap();
                jailer.jail()?;
                let child = jailer.launch().await?;
                jailer
                    .waiting_socket(tokio::time::Duration::from_secs(3))
                    .await?;

                (
                    jailer.connect().await?,
                    child,
                    LocalAsync::from_jailer(jailer, config)?,
                )
            } else {
                // Firecracker launch and connect
                let child = frck.launch().await?;
                frck.waiting_socket(tokio::time::Duration::from_secs(3))
                    .await?;
                (
                    frck.connect().await?,
                    child,
                    LocalAsync::from_frck(&frck, config)?,
                )
            };

            // Set up local environment
            local.full_clean().await;
            local.setup().await?;

            let rtck = Mutex::new(RtckAsync::from_stream(stream));

            Ok(Self {
                rtck,
                local,
                jailer,
                frck,
                config: config.clone(),
                child: Mutex::new(child),
            })
        }
    }

    #[cfg(feature = "tokio")]
    use tokio::io::{AsyncBufRead, AsyncWrite};
    impl<S: AsyncBufRead + AsyncWrite + Unpin> Machine<S> {
        /// Ping firecracker to check its soundness
        pub async fn ping_remote(&self) -> RtckResult<()> {
            let get_firecracker_version = events_async::GetFirecrackerVersion::new();
            Ok(self
                .rtck
                .lock()
                .execute(&get_firecracker_version)
                .await
                .map_err(|e| {
                    RtckError::new(
                        RtckErrorClass::RemoteError,
                        format!("Fail to ping remote {}", e.to_string()),
                    )
                })?)
        }

        /// Automatically configure the machine.
        /// User must guarantee that `config` passed to the machine contains
        /// valid firecracker configuration (`frck_config`).
        pub async fn configure(&self) -> RtckResult<()> {
            // If configuration has been exported, then the machine should have been configured.
            if self.config.frck_export_path.is_some() {
                return Ok(());
            }

            use events_async::*;

            // User must guarantee that proper
            let frck_config = self.config.frck_config.as_ref().ok_or(RtckError::new(
                RtckErrorClass::ConfigError,
                "No proper firecracker configuration passed".to_string(),
            ))?;

            // Logger
            {
                if let Some(logger) = &frck_config.logger {
                    let put_logger = PutLogger::new(logger.clone());
                    self.rtck.lock().execute(&put_logger).await?;
                    if put_logger.is_err() {
                        log::error!("[PutLogger failed, error = {}]", put_logger.get_res().err());
                    }
                }
            }

            // Metrics
            {
                if let Some(metrics) = &frck_config.metrics {
                    let put_metrics = PutMetrics::new(metrics.clone());
                    self.rtck.lock().execute(&put_metrics).await?;
                    if put_metrics.is_err() {
                        log::error!(
                            "[PutMetrics failed, error = {}]",
                            put_metrics.get_res().err()
                        );
                    }
                }
            }

            // Guest boot source
            {
                if let Some(boot_source) = &frck_config.boot_source {
                    let put_guest_boot_source = PutGuestBootSource::new(boot_source.clone());
                    self.rtck.lock().execute(&put_guest_boot_source).await?;
                    if put_guest_boot_source.is_err() {
                        log::error!(
                            "[PutGuestBootSource failed, error = {}]",
                            put_guest_boot_source.get_res().err()
                        );
                    }
                }
            }

            // Guest drives
            {
                if let Some(drives) = &frck_config.drives {
                    for drive in drives {
                        let put_guest_drive_by_id = PutGuestDriveById::new(drive.clone());
                        self.rtck.lock().execute(&put_guest_drive_by_id).await?;
                        if put_guest_drive_by_id.is_err() {
                            log::error!(
                                "[PutGuestDriveById failed, error = {}]",
                                put_guest_drive_by_id.get_res().err()
                            );
                        }
                    }
                }
            }

            // Guest network interfaces
            {
                if let Some(ifaces) = &frck_config.network_interfaces {
                    for iface in ifaces {
                        let put_guest_network_interface_by_id =
                            PutGuestNetworkInterfaceById::new(iface.clone());
                        self.rtck
                            .lock()
                            .execute(&put_guest_network_interface_by_id)
                            .await?;
                        if put_guest_network_interface_by_id.is_err() {
                            log::error!(
                                "[PutGuestNetworkInterfaceById failed, error = {}]",
                                put_guest_network_interface_by_id.get_res().err()
                            );
                        }
                    }
                }
            }

            // Vsocks
            {
                if let Some(vsocks) = &frck_config.vsock_devices {
                    for vsock in vsocks {
                        let put_guest_vsock = PutGuestVsock::new(vsock.clone());
                        self.rtck.lock().execute(&put_guest_vsock).await?;
                        if put_guest_vsock.is_err() {
                            log::error!(
                                "[PutGuestVsock failed, error = {}]",
                                put_guest_vsock.get_res().err()
                            );
                        }
                    }
                }
            }

            // CPU configuration
            {
                if let Some(cpu_config) = &frck_config.cpu_config {
                    let put_cpu_configuration = PutCpuConfiguration::new(cpu_config.clone());
                    self.rtck.lock().execute(&put_cpu_configuration).await?;
                    if put_cpu_configuration.is_err() {
                        log::error!(
                            "[PutCpuConfiguration failed, error = {}]",
                            put_cpu_configuration.get_res().err()
                        );
                    }
                }
            }

            // Machine configuration
            {
                if let Some(machine_config) = &frck_config.machine_config {
                    let put_machine_configuration =
                        PutMachineConfiguration::new(machine_config.clone());
                    self.rtck.lock().execute(&put_machine_configuration).await?;
                    if put_machine_configuration.is_err() {
                        log::error!(
                            "[PutMachineConfiguration failed, error = {}]",
                            put_machine_configuration.get_res().err()
                        );
                    }
                }
            }

            // Balloon
            {
                if let Some(balloon) = &frck_config.balloon {
                    let put_balloon = PutBalloon::new(balloon.clone());
                    self.rtck.lock().execute(&put_balloon).await?;
                    if put_balloon.is_err() {
                        log::error!(
                            "[PutBalloon failed, error = {}]",
                            put_balloon.get_res().err()
                        );
                    }
                }
            }

            // Entropy device
            {
                if let Some(entropy_device) = &frck_config.entropy_device {
                    let put_entropy = PutEntropy::new(entropy_device.clone());
                    self.rtck.lock().execute(&put_entropy).await?;
                    if put_entropy.is_err() {
                        log::error!(
                            "[PutEntropy failed, error = {}]",
                            put_entropy.get_res().err()
                        );
                    }
                }
            }

            // Initial mmds content
            {
                if let Some(content) = &frck_config.init_metadata {
                    let put_mmds = PutMmds::new(content.clone());
                    self.rtck.lock().execute(&put_mmds).await?;
                    if put_mmds.is_err() {
                        log::error!("[PutMmds failed, error = {}]", put_mmds.get_res().err());
                    }
                }
            }

            Ok(())
        }

        /// Start the machine by notifying the hypervisor
        pub async fn start(&self) -> RtckResult<()> {
            let start_machine = events_async::CreateSyncAction::new(InstanceActionInfo {
                action_type: ActionType::InstanceStart,
            });

            self.rtck.lock().execute(&start_machine).await?;
            Ok(())
        }

        /// Pause the machine by notifying the hypervisor
        pub async fn pause(&self) -> RtckResult<()> {
            let pause_machine = events_async::PatchVm::new(vm::VM_STATE_PAUSED);

            self.rtck.lock().execute(&pause_machine).await?;
            Ok(())
        }

        /// Resume the machine by notifying the hypervisor
        pub async fn resume(&self) -> RtckResult<()> {
            let resume_machine = events_async::PatchVm::new(vm::VM_STATE_RESUMED);

            self.rtck.lock().execute(&resume_machine).await?;
            Ok(())
        }

        /// Stop the machine by notifying the hypervisor
        pub async fn stop(&self) -> RtckResult<()> {
            let stop_machine = events_async::CreateSyncAction::new(InstanceActionInfo {
                action_type: ActionType::SendCtrlAtlDel,
            });

            self.rtck.lock().execute(&stop_machine).await?;
            Ok(())
        }

        /// Stop the machine forcefully by killing the firecracker process
        pub async fn stop_force(&self) -> RtckResult<()> {
            self.child.lock().kill().await.map_err(|e| {
                log::error!("[Machine::stop_force killing failed, error = {}]", e);
                RtckError::new(
                    RtckErrorClass::MachineError,
                    "Fail to kill the machine".to_string(),
                )
            })
        }

        /// Delete the machine by notifying firecracker
        pub async fn delete(&self) -> RtckResult<()> {
            // Stop the machine first
            self.stop().await?;
            let query_status = events_async::DescribeInstance::new();
            self.rtck.lock().execute(&query_status).await?;

            if query_status.is_err() {
                log::error!(
                    "[Machine::delete query status failed, error = {}]",
                    query_status.get_res().err()
                );
                return Err(RtckError::new(
                    RtckErrorClass::MachineError,
                    "Fail to query status".to_string(),
                ));
            }

            let state = query_status.get_res().succ().state;

            use crate::models::instance_info;
            if state == instance_info::State::Running {
                log::warn!("[Machine::delete cannot stop the machine, killing...]");
                self.stop_force().await?;
            }

            Ok(())
        }

        /// Delete the machine and do cleaning at the same time
        pub async fn delete_and_clean(&self) -> RtckResult<()> {
            self.delete().await?;
            self.local.full_clean().await;
            Ok(())
        }

        /// Create a snapshot
        pub async fn snapshot<P: AsRef<str>, Q: AsRef<str>>(
            &self,
            state_path: P,
            mem_path: Q,
            _type: SnapshotType,
        ) -> RtckResult<()> {
            let create_snapshot = events_async::CreateSnapshot::new(SnapshotCreateParams {
                mem_file_path: state_path.as_ref().to_string(),
                snapshot_path: mem_path.as_ref().to_string(),
                snapshot_type: Some(_type),
                version: None,
            });

            self.rtck.lock().execute(&create_snapshot).await?;
            Ok(())
        }
    }
}
