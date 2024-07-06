use crate::RtckResult;

pub trait FirecrackerRequest {
    fn to_string(&self) -> String;
}

pub trait FirecrackerResponse {
    type Payload;
    fn is_succ(&self) -> bool;
    fn is_err(&self) -> bool;
    fn decode(payload: &Option<Vec<u8>>) -> RtckResult<Self> where Self: Sized;
}

pub trait FirecrackerEvent {
    type Req: FirecrackerRequest;
    type Res: FirecrackerResponse;
    fn req(&self) -> String;
    fn decode(payload: &Option<Vec<u8>>) -> RtckResult<Self::Res>;
}

pub mod create_snapshot;
// pub mod create_sync_action;
// pub mod describe_balloon_config;
// pub mod describe_balloon_stats;
// pub mod describe_instance;
// pub mod get_export_vm_config;
pub mod get_firecracker_version;
// pub mod get_machine_configuration;
// pub mod get_mmds;

// pub mod patch_balloon_stats_interval;
// pub mod patch_balloon;
// pub mod patch_guest_drive_by_id;
// pub mod patch_guest_network_interface_by_id;
// pub mod patch_machine_configuration;
// pub mod patch_mmds;
// pub mod patch_vm;

// pub mod load_snapshot;
// pub mod put_balloon;
// pub mod put_cpu_configuration;
// pub mod put_entropy;
// pub mod put_guest_boot_source;
// pub mod put_guest_drive_by_id;
// pub mod put_guest_network_interface_by_id;
// pub mod put_guest_vsock;
// pub mod put_logger;
// pub mod put_machine_configuration;
// pub mod put_metrics;
// pub mod put_mmds_config;
// pub mod put_mmds;
