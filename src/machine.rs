pub mod machine {
    use std::io::{BufRead, Write};

    use crate::{
        events::events,
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
    }

    impl<S: BufRead + Write> Machine<S> {
        pub fn create() -> RtckResult<Self> {
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
    use tokio::io::{AsyncBufRead, AsyncWrite};

    use crate::{
        events::events_async,
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
    }

    impl<S: AsyncBufRead + AsyncWrite + Unpin> Machine<S> {
        /// Create a machine from scratch
        pub fn create() -> RtckResult<Self> {
            todo!()
        }

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
