mod machine {}

pub mod machine_async {
    use parking_lot::Mutex;
    use tokio::io::{AsyncBufRead, AsyncWrite};

    use crate::{
        events::events_async::{self, GetFirecrackerVersion},
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
        pub async fn ping_remote(&mut self) -> RtckResult<()> {
            let get_firecracker_version = GetFirecrackerVersion::new();
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
