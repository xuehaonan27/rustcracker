use tokio::sync::Mutex;

use crate::{
    agent::agent::Agent, config::GlobalConfig, firecracker::firecracker_async::FirecrackerAsync,
    jailer::jailer_async::JailerAsync, local::local_async::LocalAsync,
    reqres::GetFirecrackerVersion, RtckError, RtckResult,
};

pub struct Machine {
    agent: Agent,
    config: GlobalConfig,
    local: LocalAsync,
    frck: FirecrackerAsync,
    jailer: Option<JailerAsync>,
    child: Mutex<tokio::process::Child>,
}

impl Machine {
    /// Dump the global configuration
    pub fn get_config(&self) -> GlobalConfig {
        self.config.clone()
    }

    pub async fn create(config: GlobalConfig) -> RtckResult<Self> {
        config.validate()?;

        let frck = FirecrackerAsync::from_config(&config)?;

        let mut jailer = JailerAsync::from_config(&config).ok();

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
                LocalAsync::from_jailer(jailer, &config)?,
            )
        } else {
            // Firecracker launch and connect
            let child = frck.launch().await?;
            frck.waiting_socket(tokio::time::Duration::from_secs(3))
                .await?;
            (
                frck.connect().await?,
                child,
                LocalAsync::from_frck(&frck, &config)?,
            )
        };

        // Set up local environment
        local.full_clean().await;
        local.setup().await?;
        let lock = local.create_lock()?;

        let agent = Agent::from_stream_lock(stream, lock);

        Ok(Self {
            agent,
            local,
            jailer,
            frck,
            config: config.clone(),
            child: Mutex::new(child),
        })
    }

    /// Ping firecracker to check its soundness
    pub async fn ping_remote(&mut self) -> RtckResult<()> {
        let event = GetFirecrackerVersion::new();
        let _ = self.agent.event(event).await?;
        Ok(())
    }

    /// Automatically configure the machine.
    /// User must guarantee that `config` passed to the machine contains
    /// valid firecracker configuration (`frck_config`).
    pub async fn configure(&self) -> RtckResult<()> {
        // If configuration has been exported, then the machine should have been configured.
        if self.config.frck_export_path.is_some() {
            return Ok(());
        }

        // User must guarantee that proper
        let frck_config = self
            .config
            .frck_config
            .as_ref()
            .ok_or(RtckError::Config("no firecracker config".to_string()))?;

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
}
