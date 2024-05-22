use serde::{Deserialize, Serialize};

use crate::{
    models::{
        balloon::Balloon,
        balloon_stats::BalloonStatistics,
        balloon_stats_update::BalloonStatsUpdate,
        balloon_update::BalloonUpdate,
        boot_source::BootSource,
        cpu_template::{CPUConfig, CPUTemplate},
        drive::Drive,
        entropy_device::EntropyDevice,
        error::InternalError,
        firecracker_version::FirecrackerVersion,
        full_vm_configuration::FullVmConfiguration,
        instance_action_info::InstanceActionInfo,
        instance_info::InstanceInfo,
        kernel_args::KernelArgs,
        logger::Logger,
        machine_configuration::MachineConfiguration,
        memory_backend::MemoryBackend,
        metrics::Metrics,
        mmds_config::{MmdsConfig, MmdsContentsObject},
        network_interface::NetworkInterface,
        partial_drive::PartialDrive,
        partial_network_interface::PartialNetworkInterface,
        rate_limiter::RateLimiter,
        snapshot_create_params::SnapshotCreateParams,
        snapshot_load_params::SnapshotLoadParams,
        token_bucket::TokenBucket,
        vm::Vm,
        vsock::Vsock,
    },
    RtckResult,
};

pub(crate) trait Serde {
    fn encode(&self) -> RtckResult<String>;

    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized;
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Empty {}

impl Serde for Empty {
    fn decode<S: AsRef<str>>(_line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(Empty {})
    }
    fn encode(&self) -> RtckResult<String> {
        Ok(String::new())
    }
}

impl Serde for BalloonStatsUpdate {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for BalloonStatistics {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for BalloonUpdate {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for Balloon {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for BootSource {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for CPUTemplate {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for CPUConfig {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for Drive {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for EntropyDevice {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for InternalError {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for FirecrackerVersion {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for FullVmConfiguration {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for InstanceActionInfo {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for InstanceInfo {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for KernelArgs {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for Logger {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for MachineConfiguration {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for MemoryBackend {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for Metrics {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for MmdsConfig {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for MmdsContentsObject {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for NetworkInterface {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for PartialDrive {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for PartialNetworkInterface {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for RateLimiter {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for SnapshotCreateParams {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for SnapshotLoadParams {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for TokenBucket {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for Vm {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Serde for Vsock {
    fn decode<S: AsRef<str>>(line: &S) -> RtckResult<Self>
    where
        Self: Sized,
    {
        Ok(serde_json::from_str(line.as_ref())?)
    }

    fn encode(&self) -> RtckResult<String> {
        Ok(serde_json::to_string(&self)?)
    }
}
