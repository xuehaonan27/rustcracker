pub mod machine {
    use std::io::{BufRead, Write};

    use crate::{
        config::GlobalConfig,
        events::events,
        local::{firecracker::Firecracker, jailer::Jailer, local::Local},
        models::{
            instance_action_info::{ActionType, InstanceActionInfo},
            snapshot_create_params::{SnapshotCreateParams, SnapshotType},
            vm,
        },
        rtck::Rtck,
        RtckError, RtckErrorClass, RtckResult,
    };

    pub struct Machine<S> {
        rtck: Rtck<S>,
        local: Local,
        jailer: Jailer,
        frck: Firecracker,
    }

    impl<S: BufRead + Write> Machine<S> {
        pub fn create(config: &GlobalConfig) -> RtckResult<Self> {
            todo!()
        }

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

        pub fn start(&mut self) -> RtckResult<()> {
            let mut start_machine = events::CreateSyncAction::new(InstanceActionInfo {
                action_type: ActionType::InstanceStart,
            });

            self.rtck.execute(&mut start_machine)?;
            Ok(())
        }

        pub fn pause(&mut self) -> RtckResult<()> {
            let mut pause_machine = events::PatchVm::new(vm::Vm {
                state: vm::State::Paused,
            });

            self.rtck.execute(&mut pause_machine)?;
            Ok(())
        }

        pub fn resume(&mut self) -> RtckResult<()> {
            let mut resume_machine = events::PatchVm::new(vm::Vm {
                state: vm::State::Resumed,
            });

            self.rtck.execute(&mut resume_machine)?;
            Ok(())
        }

        pub async fn stop(&mut self) -> RtckResult<()> {
            let mut stop_machine = events::CreateSyncAction::new(InstanceActionInfo {
                action_type: ActionType::SendCtrlAtlDel,
            });

            self.rtck.execute(&mut stop_machine)?;
            Ok(())
        }

        pub fn delete(&mut self) -> RtckResult<()> {
            todo!()
        }

        pub fn snapshot<P: AsRef<str>, Q: AsRef<str>>(
            &mut self,
            state_path: P,
            mem_path: Q,
            _type: SnapshotType,
        ) -> RtckResult<()> {
            let mut create_snapshot = events::CreateSnapshot::new(SnapshotCreateParams {
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
    use tokio::{
        io::{AsyncBufRead, AsyncWrite, BufStream},
        net::UnixStream,
        time,
    };

    use crate::{
        config::{FirecrackerConfig, GlobalConfig},
        events::events_async,
        local::{
            firecracker_async::FirecrackerAsync, handle_entry, jailer_async::JailerAsync,
            local_async::LocalAsync,
        },
        models::{
            instance_action_info::{ActionType, InstanceActionInfo},
            snapshot_create_params::{SnapshotCreateParams, SnapshotType},
            vm,
        },
        ops_res::{
            put_balloon, put_cpu_configuration, put_entropy, put_guest_boot_source,
            put_guest_drive_by_id, put_guest_network_interface_by_id, put_guest_vsock,
            put_mmds_config,
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
    }

    impl Machine<BufStream<UnixStream>> {
        /// Create a machine from scratch, using default stream
        pub async fn create(config: &GlobalConfig) -> RtckResult<Self> {
            config.validate()?;

            let frck = FirecrackerAsync::from_config(config)?;
            let mut jailer = JailerAsync::from_config(config).ok();
            let frck_config = config.frck_config.clone().unwrap();

            if config.frck_export.is_some() {
                config.export_config_async().await?;
            }

            let (stream, local) = if config.using_jailer.unwrap() {
                // Set up for jailing
                assert!(jailer.is_some());
                let jailer = jailer.as_mut().unwrap();
                jailer.jail()?;
                jailer.launch().await?;
                jailer.waiting_socket(time::Duration::from_secs(3)).await?;

                (jailer.connect().await?, LocalAsync::from_jailer(jailer)?)
            } else {
                // Firecracker launch and connect
                frck.launch().await?;
                frck.waiting_socket(time::Duration::from_secs(3)).await?;
                (frck.connect().await?, LocalAsync::from_frck(&frck)?)
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
            })
        }
    }

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
            if self.config.frck_export.is_some() {
                return Ok(());
            }

            use crate::models::*;
            use events_async::*;

            // User must guarantee that proper
            let frck_config = self.config.frck_config.as_ref().ok_or(RtckError::new(
                RtckErrorClass::ConfigError,
                "No proper firecracker configuration passed".to_string(),
            ))?;

            // Logger
            {
                // let put_logger = PutLogger::new(logger::Logger {
                //     level: frck_config.log_level,
                //     log_path: frck_config.log_path,
                //     show_level: frck_config.lo
                // });
            }

            // Guest boot source
            {
                if let Some(kernel_image_path) = &frck_config.kernel_image_path {
                    let put_guest_boot_source = PutGuestBootSource::new(boot_source::BootSource {
                        boot_args: frck_config.kernel_args.clone(),
                        initrd_path: frck_config.initrd_path.clone(),
                        kernel_image_path: kernel_image_path.clone(),
                    });
                    self.rtck.lock().execute(&put_guest_boot_source).await?;
                    if put_guest_boot_source.is_err() {
                        log::error!(
                            "[PutGuestBootSource failed, error = {}]",
                            put_guest_boot_source.get_res().err()
                        );
                    }
                }
            }

            // CPU configuration
            {
                // let put_cpu_configuration = PutCpuConfiguration::new(cpu_template::CPUConfig {
                //     cpuid_modifiers: (),
                //     msr_modifiers: (),
                //     reg_modifiers: (),
                // });
            }

            // Machine configuration
            {
                if let Some(machine_cfg) = &frck_config.machine_cfg {
                    let put_machine_configuration =
                        PutMachineConfiguration::new(machine_cfg.clone());
                    self.rtck.lock().execute(&put_machine_configuration).await?;
                    if put_machine_configuration.is_err() {
                        log::error!(
                            "[PutMachineConfiguration failed, error = {}]",
                            put_machine_configuration.get_res().err()
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

            // Metrics
            {
                if let Some(path) = &frck_config.metrics_path {
                    let put_metrics = PutMetrics::new(metrics::Metrics {
                        metrics_path: path.clone(),
                    });
                    self.rtck.lock().execute(&put_metrics).await?;
                    if put_metrics.is_err() {
                        log::error!(
                            "[PutMetrics failed, error = {}]",
                            put_metrics.get_res().err()
                        );
                    }
                }
            }

            {
                if let Some(content) = &frck_config.init_metadata {
                    let put_mmds = PutMmds::new(content.clone());
                    self.rtck.lock().execute(&put_mmds).await?;
                    if put_mmds.is_err() {
                        log::error!(
                            "[PutMmds failed, error = {}]",
                            put_mmds.get_res().err()
                        );
                    }
                }
            }

            // Entropy device
            {
                // let put_entropy = PutEntropy::new(data)
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
            let pause_machine = events_async::PatchVm::new(vm::Vm {
                state: vm::State::Paused,
            });

            self.rtck.lock().execute(&pause_machine).await?;
            Ok(())
        }

        /// Resume the machine by notifying the hypervisor
        pub async fn resume(&self) -> RtckResult<()> {
            let resume_machine = events_async::PatchVm::new(vm::Vm {
                state: vm::State::Resumed,
            });

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

        pub async fn delete(&self) -> RtckResult<()> {
            todo!()
        }

        /// Delete the machine and do cleaning at the same time
        pub async fn delete_clean(&self) -> RtckResult<()> {
            todo!()
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
