use tokio::sync::Mutex;

use crate::{
    agent::agent::Agent, config::GlobalConfig, firecracker::FirecrackerAsync, jailer::JailerAsync,
    local::LocalAsync, models::*, reqres::*, RtckError, RtckResult,
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
    pub async fn configure(&mut self) -> RtckResult<()> {
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
                let res = self.agent.event(put_logger).await?;
                if res.is_err() {
                    log::error!("PutLogger failed");
                }
            }
        }

        // Metrics
        {
            if let Some(metrics) = &frck_config.metrics {
                let put_metrics = PutMetrics::new(metrics.clone());
                let res = self.agent.event(put_metrics).await?;
                if res.is_err() {
                    log::error!("PutMetrics failed");
                }
            }
        }

        // Guest boot source
        {
            if let Some(boot_source) = &frck_config.boot_source {
                let put_guest_boot_source = PutGuestBootSource::new(boot_source.clone());
                let res = self.agent.event(put_guest_boot_source).await?;
                if res.is_err() {
                    log::error!("PutGuestBootSource failed");
                }
            }
        }

        // Guest drives
        {
            if let Some(drives) = &frck_config.drives {
                for drive in drives {
                    let put_guest_drive_by_id = PutGuestDriveByID::new(drive.clone());
                    let res = self.agent.event(put_guest_drive_by_id).await?;
                    if res.is_err() {
                        log::error!("PutGuestDriveById failed");
                    }
                }
            }
        }

        // Guest network interfaces
        {
            if let Some(ifaces) = &frck_config.network_interfaces {
                for iface in ifaces {
                    let put_guest_network_interface_by_id =
                        PutGuestNetworkInterfaceByID::new(iface.clone());
                    let res = self.agent.event(put_guest_network_interface_by_id).await?;
                    if res.is_err() {
                        log::error!("PutGuestNetworkInterfaceById failed");
                    }
                }
            }
        }

        // Vsocks
        {
            if let Some(vsocks) = &frck_config.vsock_devices {
                for vsock in vsocks {
                    let put_guest_vsock = PutGuestVsock::new(vsock.clone());
                    let res = self.agent.event(put_guest_vsock).await?;
                    if res.is_err() {
                        log::error!("PutGuestVsock failed");
                    }
                }
            }
        }

        // CPU configuration
        {
            if let Some(cpu_config) = &frck_config.cpu_config {
                let put_cpu_configuration = PutCpuConfiguration::new(cpu_config.clone());
                let res = self.agent.event(put_cpu_configuration).await?;
                if res.is_err() {
                    log::error!("PutCpuConfiguration failed");
                }
            }
        }

        // Machine configuration
        {
            if let Some(machine_config) = &frck_config.machine_config {
                let put_machine_configuration =
                    PutMachineConfiguration::new(machine_config.clone());
                let res = self.agent.event(put_machine_configuration).await?;
                if res.is_err() {
                    log::error!("PutMachineConfiguration failed");
                }
            }
        }

        // Balloon
        {
            if let Some(balloon) = &frck_config.balloon {
                let put_balloon = PutBalloon::new(balloon.clone());
                let res = self.agent.event(put_balloon).await?;
                if res.is_err() {
                    log::error!("PutBalloon failed");
                }
            }
        }

        // Entropy device
        {
            if let Some(entropy_device) = &frck_config.entropy_device {
                let put_entropy = PutEntropy::new(entropy_device.clone());
                let res = self.agent.event(put_entropy).await?;
                if res.is_err() {
                    log::error!("PutEntropy failed");
                }
            }
        }

        // Initial mmds content
        {
            if let Some(content) = &frck_config.init_metadata {
                let put_mmds = PutMmds::new(content.clone());
                let res = self.agent.event(put_mmds).await?;
                if res.is_err() {
                    log::error!("PutMmds failed");
                }
            }
        }

        Ok(())
    }

    /// Start the machine by notifying the hypervisor
    pub async fn start(&mut self) -> RtckResult<()> {
        let start_machine = CreateSyncAction::new(InstanceActionInfo {
            action_type: ActionType::InstanceStart,
        });

        let res = self.agent.event(start_machine).await?;
        if res.is_err() {
            log::error!("Machine::start fail");
            return Err(RtckError::Machine("fail to start".to_string()));
        }
        Ok(())
    }

    /// Pause the machine by notifying the hypervisor
    pub async fn pause(&mut self) -> RtckResult<()> {
        let pause_machine = PatchVm::new(vm::VM_STATE_PAUSED);

        let res = self.agent.event(pause_machine).await?;
        if res.is_err() {
            log::error!("Machine::pause fail");
            return Err(RtckError::Machine("fail to pause".to_string()));
        }
        Ok(())
    }

    /// Resume the machine by notifying the hypervisor
    pub async fn resume(&mut self) -> RtckResult<()> {
        let resume_machine = PatchVm::new(vm::VM_STATE_RESUMED);

        let res = self.agent.event(resume_machine).await?;
        if res.is_err() {
            log::error!("Machine::resume fail");
            return Err(RtckError::Machine("fail to resume".to_string()));
        }
        Ok(())
    }

    /// Stop the machine by notifying the hypervisor
    pub async fn stop(&mut self) -> RtckResult<()> {
        let stop_machine = CreateSyncAction::new(InstanceActionInfo {
            action_type: ActionType::SendCtrlAtlDel,
        });

        let res = self.agent.event(stop_machine).await?;
        if res.is_err() {
            log::error!("Machine::stop fail");
            return Err(RtckError::Machine("fail to stop".to_string()));
        }
        Ok(())
    }

    /// Stop the machine forcefully by killing the firecracker process
    pub async fn stop_force(&mut self) -> RtckResult<()> {
        self.child.lock().await.kill().await.map_err(|e| {
            log::error!("Machine::stop_force killing failed, error = {}", e);
            RtckError::Machine("fail to kill the machine".to_string())
        })
    }

    /// Delete the machine by notifying firecracker
    pub async fn delete(&mut self) -> RtckResult<()> {
        // Stop the machine first
        self.stop().await?;
        let query_status = DescribeInstance::new();
        let res = self.agent.event(query_status).await?;

        if res.is_err() {
            log::error!("Machine::delete query status failed");
            return Err(RtckError::Machine("fail to query status".to_string()));
        }

        let state = res.succ().state;

        use crate::models::instance_info;
        if state == instance_info::State::Running {
            log::warn!("[Machine::delete cannot stop the machine, killing...]");
            self.stop_force().await?;
        }

        Ok(())
    }

    /// Delete the machine and do cleaning at the same time
    pub async fn delete_and_clean(&mut self) -> RtckResult<()> {
        self.delete().await?;
        self.local.full_clean().await;
        Ok(())
    }

    /// Create a snapshot
    pub async fn snapshot<P: AsRef<str>, Q: AsRef<str>>(
        &mut self,
        state_path: P,
        mem_path: Q,
        _type: SnapshotType,
    ) -> RtckResult<()> {
        let create_snapshot = CreateSnapshot::new(SnapshotCreateParams {
            mem_file_path: state_path.as_ref().to_string(),
            snapshot_path: mem_path.as_ref().to_string(),
            snapshot_type: Some(_type),
            version: None,
        });

        let res = self.agent.event(create_snapshot).await?;
        if res.is_err() {
            log::error!("Machine::snapshot fail");
            return Err(RtckError::Machine("fail to create snapshot".to_string()));
        }
        Ok(())
    }
}
