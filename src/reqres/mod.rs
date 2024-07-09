use crate::RtckResult;
use serde::{Deserialize, Serialize};

pub trait FirecrackerRequest {
    fn encode(&self) -> String;
}

pub trait FirecrackerResponse {
    type Succ<'de>: Serialize + Deserialize<'de>;
    type Fail<'de>: Serialize + Deserialize<'de>;
    // type Payload = Either<Self::Succ, Self::Fail>;
    fn is_succ(&self) -> bool;
    fn is_err(&self) -> bool;
    fn succ<'de>(&self) -> &Self::Succ<'de>;
    fn err<'de>(&self) -> &Self::Fail<'de>;
    fn create_succ<'de>(content: Self::Succ<'de>) -> Self
    where
        Self: Sized;
    fn create_fail<'de>(content: Self::Fail<'de>) -> Self
    where
        Self: Sized;
    // fn decode(payload: &Option<Vec<u8>>) -> RtckResult<Self> where Self: Sized;
    fn decode<'de>(payload: &'de Vec<u8>) -> crate::RtckResult<Self>
    where
        Self: Sized,
        Self::Succ<'de>: 'static,
    {
        match serde_json::from_slice::<Self::Succ<'de>>(&payload) {
            Ok(content) => Ok(Self::create_succ(content)),
            Err(_) => match serde_json::from_slice::<Self::Fail<'de>>(&payload) {
                Ok(content) => Ok(Self::create_fail(content)),
                Err(e) => Err(crate::RtckError::Decode(e.to_string())),
            },
        }
    }
}

macro_rules! impl_all_firecracker_traits {
    // with body
    ($struct_name:ty, $method:expr, $endpoint:expr, $payload:ty, $res_succ:ty, $res_fail:ty) => {
        use super::{FirecrackerEvent, FirecrackerRequest, FirecrackerResponse};
        use crate::{
            agent::{serialize_request, HttpRequest},
            models::{$payload, $res_fail, $res_succ},
        };
        use either::Either;

        impl paste::paste! {[<$struct_name Request>]} {
            pub fn new(payload: $payload) -> Self {
                Self(payload)
            }
        }

        impl $struct_name {
            pub fn new(payload: $payload) -> Self {
                Self(<paste::paste! {[<$struct_name Request>]}>::new(payload))
            }
        }

        impl_firecracker_request!(
            paste::paste! {[<$struct_name Request>]},
            $method,
            $endpoint,
            $payload
        );
        impl_firecracker_response!(
            paste::paste! {[<$struct_name Response>]},
            $res_succ,
            $res_fail
        );
        impl_firecracker_event!(
            $struct_name,
            paste::paste! {[<$struct_name Request>]},
            paste::paste! {[<$struct_name Response>]}
        );
    };
    // without body
    ($struct_name:ty, $method:expr, $endpoint:expr, $res_succ:ty, $res_fail:ty) => {
        use super::{FirecrackerEvent, FirecrackerRequest, FirecrackerResponse};
        use crate::{
            agent::{serialize_request, HttpRequest},
            models::{$res_fail, $res_succ},
        };
        use either::Either;

        impl paste::paste! {[<$struct_name Request>]} {
            pub fn new() -> Self {
                Self
            }
        }

        impl $struct_name {
            pub fn new() -> Self {
                Self(paste::paste! {[<$struct_name Request>]})
            }
        }

        impl_firecracker_request!(paste::paste! {[<$struct_name Request>]}, $method, $endpoint);
        impl_firecracker_response!(
            paste::paste! {[<$struct_name Response>]},
            $res_succ,
            $res_fail
        );
        impl_firecracker_event!(
            $struct_name,
            paste::paste! {[<$struct_name Request>]},
            paste::paste! {[<$struct_name Response>]}
        );
    };
    // with body and id
    ($struct_name:ty, $method:expr, $endpoint:expr, $payload:ty, $id:ident, $res_succ:ty, $res_fail:ty) => {
        use super::{FirecrackerEvent, FirecrackerRequest, FirecrackerResponse};
        use crate::{
            agent::{serialize_request, HttpRequest},
            models::{$payload, $res_fail, $res_succ},
        };
        use either::Either;

        impl paste::paste! {[<$struct_name Request>]} {
            pub fn new(payload: $payload) -> Self {
                Self(payload)
            }
        }

        impl $struct_name {
            pub fn new(payload: $payload) -> Self {
                Self(<paste::paste! {[<$struct_name Request>]}>::new(payload))
            }
        }

        impl_firecracker_request!(
            paste::paste! {[<$struct_name Request>]},
            $method,
            $endpoint,
            $payload,
            $id
        );
        impl_firecracker_response!(
            paste::paste! {[<$struct_name Response>]},
            $res_succ,
            $res_fail
        );
        impl_firecracker_event!(
            $struct_name,
            paste::paste! {[<$struct_name Request>]},
            paste::paste! {[<$struct_name Response>]}
        );
    };
}

macro_rules! impl_firecracker_request {
    // with body
    ($struct_name:ty, $method:expr, $endpoint:expr, $payload:ty) => {
        impl FirecrackerRequest for $struct_name {
            fn encode(&self) -> String {
                let body = serde_json::to_string(&self.0).expect("Fatal error");
                let request = HttpRequest::new($method, $endpoint, Some(body.len()), Some(body));
                serialize_request(&request)
            }
        }
    };

    // without body
    ($struct_name:ty, $method:expr, $endpoint:expr) => {
        impl FirecrackerRequest for $struct_name {
            fn encode(&self) -> String {
                let request = HttpRequest::new($method, $endpoint, None, None);
                serialize_request(&request)
            }
        }
    };

    // with body and id
    ($struct_name:ty, $method:expr, $endpoint:expr, $payload:ty, $id:ident) => {
        impl FirecrackerRequest for $struct_name {
            fn encode(&self) -> String {
                let path = format!("{}/{}", $endpoint, &self.0.$id);
                let body = serde_json::to_string(&self.0).expect("Fatal error");
                let request =
                    HttpRequest::new($method, path.as_str(), Some(body.len()), Some(body));
                serialize_request(&request)
            }
        }
    };
}

macro_rules! impl_firecracker_event {
    ($type:ty, $req:ty, $res:ty) => {
        impl FirecrackerEvent for $type {
            type Req = $req;
            type Res = $res;

            fn req(&self) -> String {
                self.0.encode()
            }

            fn decode(payload: &Vec<u8>) -> crate::RtckResult<Self::Res> {
                Self::Res::decode(payload)
            }
        }
    };
}

macro_rules! impl_firecracker_response {
    ($type:ty, $succ:ty, $fail:ty) => {
        impl FirecrackerResponse for $type {
            type Succ<'de> = $succ;
            type Fail<'de> = $fail;

            #[inline]
            fn is_succ(&self) -> bool {
                self.0.is_left()
            }

            #[inline]
            fn is_err(&self) -> bool {
                self.0.is_right()
            }

            #[inline]
            fn succ<'de>(&self) -> &Self::Succ<'de> {
                self.0
                    .as_ref()
                    .left()
                    .expect("Use is_succ to check your response")
            }

            #[inline]
            fn err<'de>(&self) -> &Self::Fail<'de> {
                self.0
                    .as_ref()
                    .right()
                    .expect("Use is_err to check your response")
            }

            #[inline]
            fn create_succ<'de>(content: Self::Succ<'de>) -> Self
            where
                Self: Sized,
            {
                Self(either::Left(content))
            }

            #[inline]
            fn create_fail<'de>(content: Self::Fail<'de>) -> Self
            where
                Self: Sized,
            {
                Self(either::Right(content))
            }
        }
    };
}

pub trait FirecrackerEvent {
    type Req: FirecrackerRequest;
    type Res: FirecrackerResponse;
    fn req(&self) -> String;
    fn decode(payload: &Vec<u8>) -> RtckResult<Self::Res>;
}

pub mod create_snapshot;
pub use create_snapshot::CreateSnapshot;
pub mod create_sync_action;
pub use create_sync_action::CreateSyncAction;
pub mod describe_balloon_config;
pub use describe_balloon_config::DescribeBalloonConfig;
pub mod describe_balloon_stats;
pub use describe_balloon_stats::DescribeBalloonStats;
pub mod describe_instance;
pub use describe_instance::DescribeInstance;
pub mod get_export_vm_config;
pub use get_export_vm_config::GetExportVmConfig;
pub mod get_firecracker_version;
pub use get_firecracker_version::GetFirecrackerVersion;
pub mod get_machine_configuration;
pub use get_machine_configuration::GetMachineConfiguration;
pub mod get_mmds;
pub use get_mmds::GetMmds;

pub mod patch_balloon_stats_interval;
pub use patch_balloon_stats_interval::PatchBalloonStatsInterval;
pub mod patch_balloon;
pub use patch_balloon::PatchBalloon;
pub mod patch_guest_drive_by_id;
pub use patch_guest_drive_by_id::PatchGuestDriveByID;
pub mod patch_guest_network_interface_by_id;
pub use patch_guest_network_interface_by_id::PatchGuestNetworkInterfaceByID;
pub mod patch_machine_configuration;
pub use patch_machine_configuration::PatchMachineConfiguration;
pub mod patch_mmds;
pub use patch_mmds::PatchMmds;
pub mod patch_vm;
pub use patch_vm::PatchVm;

pub mod load_snapshot;
pub use load_snapshot::LoadSnapshot;
pub mod put_balloon;
pub use put_balloon::PutBalloon;
pub mod put_cpu_configuration;
pub use put_cpu_configuration::PutCpuConfiguration;
pub mod put_entropy;
pub use put_entropy::PutEntropy;
pub mod put_guest_boot_source;
pub use put_guest_boot_source::PutGuestBootSource;
pub mod put_guest_drive_by_id;
pub use put_guest_drive_by_id::PutGuestDriveByID;
pub mod put_guest_network_interface_by_id;
pub use put_guest_network_interface_by_id::PutGuestNetworkInterfaceByID;
pub mod put_guest_vsock;
pub use put_guest_vsock::PutGuestVsock;
pub mod put_logger;
pub use put_logger::PutLogger;
pub mod put_machine_configuration;
pub use put_machine_configuration::PutMachineConfiguration;
pub mod put_metrics;
pub use put_metrics::PutMetrics;
pub mod put_mmds_config;
pub use put_mmds_config::PutMmdsConfig;
pub mod put_mmds;
pub use put_mmds::PutMmds;
