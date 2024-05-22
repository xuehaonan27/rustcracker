use std::sync::Mutex;

use crate::{
    models::{
        balloon::Balloon,
        balloon_stats_update::BalloonStatsUpdate,
        balloon_update::BalloonUpdate,
        boot_source::BootSource,
        cpu_template::CPUConfig,
        drive::Drive,
        entropy_device::EntropyDevice,
        instance_action_info::InstanceActionInfo,
        logger::Logger,
        machine_configuration::MachineConfiguration,
        metrics::Metrics,
        mmds_config::{MmdsConfig, MmdsContentsObject},
        network_interface::NetworkInterface,
        partial_network_interface::PartialNetworkInterface,
        snapshot_create_params::SnapshotCreateParams,
        snapshot_load_params::SnapshotLoadParams,
        vm::Vm,
        vsock::Vsock,
    },
    ops_res::{
        create_snapshot::{CreateSnapshotOps, CreateSnapshotRes},
        create_sync_action::{CreateSyncActionOps, CreateSyncActionRes},
        describe_balloon_config::{DescribeBalloonConfigOps, DescribeBalloonConfigRes},
        describe_balloon_stats::{DescribeBalloonStatsOps, DescribeBalloonStatsRes},
        describe_instance::{DescribeInstanceOps, DescribeInstanceRes},
        get_export_vm_config::{GetExportVmConfigOps, GetExportVmConfigRes},
        get_firecracker_version::{GetFirecrackerVersionOps, GetFirecrackerVersionRes},
        get_machine_configuration::{GetMachineConfigurationOps, GetMachineConfigurationRes},
        get_mmds::{GetMmdsOps, GetMmdsRes},
        load_snapshot::{LoadSnapshotOps, LoadSnapshotRes},
        patch_balloon::{PatchBalloonOps, PatchBalloonRes},
        patch_balloon_stats_interval::{
            PatchBalloonStatsIntervalOps, PatchBalloonStatsIntervalRes,
        },
        patch_guest_drive_by_id::{PatchGuestDriveByIdOps, PatchGuestDriveByIdRes},
        patch_guest_network_interface_by_id::{
            PatchGuestNetworkInterfaceByIdOps, PatchGuestNetworkInterfaceByIdRes,
        },
        patch_machine_configuration::{PatchMachineConfigurationOps, PatchMachineConfigurationRes},
        patch_mmds::{PatchMmdsOps, PatchMmdsRes},
        patch_vm::{PatchVmOps, PatchVmRes},
        put_balloon::{PutBalloonOps, PutBalloonRes},
        put_cpu_configuration::{PutCpuConfigurationOps, PutCpuConfigurationRes},
        put_entropy::{PutEntropyOps, PutEntropyRes},
        put_guest_boot_source::{PutGuestBootSourceOps, PutGuestBootSourceRes},
        put_guest_drive_by_id::{PutGuestDriveByIdOps, PutGuestDriveByIdRes},
        put_guest_network_interface_by_id::{
            PutGuestNetworkInterfaceByIdOps, PutGuestNetworkInterfaceByIdRes,
        },
        put_guest_vsock::{PutGuestVsockOps, PutGuestVsockRes},
        put_logger::{PutLoggerOps, PutLoggerRes},
        put_machine_configuration::{PutMachineConfigurationOps, PutMachineConfigurationRes},
        put_metrics::{PutMetricsOps, PutMetricsRes},
        put_mmds::{PutMmdsOps, PutMmdsRes},
        put_mmds_config::{PutMmdsConfigOps, PutMmdsConfigRes},
        Operation, Response,
    },
    RtckResult,
};

pub trait Event<O: Operation, R: Response> {
    fn get_ops(&self) -> &O;

    fn get_ops_mut(&mut self) -> &mut O;

    fn get_res_mut(&mut self) -> RtckResult<&mut R>;

    fn set_res(&self, res: R) -> RtckResult<()>;

    fn is_succ(&mut self) -> RtckResult<bool> {
        Ok(self.get_res_mut()?.is_succ())
    }

    fn is_err(&mut self) -> RtckResult<bool> {
        Ok(self.get_res_mut()?.is_err())
    }
}

/*------------------------------ create_snapshot ------------------------------ */
pub struct CreateSnapshot {
    ops: CreateSnapshotOps,
    res: Mutex<CreateSnapshotRes>,
}

impl Event<CreateSnapshotOps, CreateSnapshotRes> for CreateSnapshot {
    fn get_ops(&self) -> &CreateSnapshotOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut CreateSnapshotOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut CreateSnapshotRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: CreateSnapshotRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl CreateSnapshot {
    pub fn new(data: SnapshotCreateParams) -> Self {
        Self {
            ops: CreateSnapshotOps::new(data),
            res: Mutex::new(CreateSnapshotRes::blank()),
        }
    }
}

/*------------------------------ create_sync_action ------------------------------ */
pub struct CreateSyncAction {
    ops: CreateSyncActionOps,
    res: Mutex<CreateSyncActionRes>,
}

impl Event<CreateSyncActionOps, CreateSyncActionRes> for CreateSyncAction {
    fn get_ops(&self) -> &CreateSyncActionOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut CreateSyncActionOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut CreateSyncActionRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: CreateSyncActionRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl CreateSyncAction {
    pub fn new(data: InstanceActionInfo) -> Self
    where
        Self: Sized,
    {
        Self {
            ops: CreateSyncActionOps::new(data),
            res: Mutex::new(CreateSyncActionRes::blank()),
        }
    }
}

/*------------------------------ describe_balloon_config ------------------------------ */
pub struct DescribeBalloonConfig {
    ops: DescribeBalloonConfigOps,
    res: Mutex<DescribeBalloonConfigRes>,
}

impl Event<DescribeBalloonConfigOps, DescribeBalloonConfigRes> for DescribeBalloonConfig {
    fn get_ops(&self) -> &DescribeBalloonConfigOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut DescribeBalloonConfigOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut DescribeBalloonConfigRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: DescribeBalloonConfigRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl DescribeBalloonConfig {
    pub fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            ops: DescribeBalloonConfigOps::new(),
            res: Mutex::new(DescribeBalloonConfigRes::blank()),
        }
    }
}

/*------------------------------ describe_balloon_stats ------------------------------ */
pub struct DescribeBalloonStats {
    ops: DescribeBalloonStatsOps,
    res: Mutex<DescribeBalloonStatsRes>,
}

impl Event<DescribeBalloonStatsOps, DescribeBalloonStatsRes> for DescribeBalloonStats {
    fn get_ops(&self) -> &DescribeBalloonStatsOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut DescribeBalloonStatsOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut DescribeBalloonStatsRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: DescribeBalloonStatsRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl DescribeBalloonStats {
    pub fn new() -> Self {
        Self {
            ops: DescribeBalloonStatsOps::new(),
            res: Mutex::new(DescribeBalloonStatsRes::blank()),
        }
    }
}

/*------------------------------ describe_instance ------------------------------ */
pub struct DescribeInstance {
    ops: DescribeInstanceOps,
    res: Mutex<DescribeInstanceRes>,
}

impl Event<DescribeInstanceOps, DescribeInstanceRes> for DescribeInstance {
    fn get_ops(&self) -> &DescribeInstanceOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut DescribeInstanceOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut DescribeInstanceRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: DescribeInstanceRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl DescribeInstance {
    pub fn new() -> Self {
        Self {
            ops: DescribeInstanceOps::new(),
            res: Mutex::new(DescribeInstanceRes::blank()),
        }
    }
}

/*------------------------------ get_export_vm_config ------------------------------ */
pub struct GetExportVmConfig {
    ops: GetExportVmConfigOps,
    res: Mutex<GetExportVmConfigRes>,
}

impl Event<GetExportVmConfigOps, GetExportVmConfigRes> for GetExportVmConfig {
    fn get_ops(&self) -> &GetExportVmConfigOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut GetExportVmConfigOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut GetExportVmConfigRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: GetExportVmConfigRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl GetExportVmConfig {
    pub fn new() -> Self {
        Self {
            ops: GetExportVmConfigOps::new(),
            res: Mutex::new(GetExportVmConfigRes::blank()),
        }
    }
}

/*------------------------------ get_firecracker_version ------------------------------ */
pub struct GetFirecrackerVersion {
    ops: GetFirecrackerVersionOps,
    res: Mutex<GetFirecrackerVersionRes>,
}

impl Event<GetFirecrackerVersionOps, GetFirecrackerVersionRes> for GetFirecrackerVersion {
    fn get_ops(&self) -> &GetFirecrackerVersionOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut GetFirecrackerVersionOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut GetFirecrackerVersionRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: GetFirecrackerVersionRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl GetFirecrackerVersion {
    pub fn new() -> Self {
        Self {
            ops: GetFirecrackerVersionOps::new(),
            res: Mutex::new(GetFirecrackerVersionRes::blank()),
        }
    }
}

/*------------------------------ get_machine_configuration ------------------------------ */
pub struct GetMachineConfiguration {
    ops: GetMachineConfigurationOps,
    res: Mutex<GetMachineConfigurationRes>,
}

impl Event<GetMachineConfigurationOps, GetMachineConfigurationRes> for GetMachineConfiguration {
    fn get_ops(&self) -> &GetMachineConfigurationOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut GetMachineConfigurationOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut GetMachineConfigurationRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: GetMachineConfigurationRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl GetMachineConfiguration {
    pub fn new() -> Self {
        Self {
            ops: GetMachineConfigurationOps::new(),
            res: Mutex::new(GetMachineConfigurationRes::blank()),
        }
    }
}

/*------------------------------ get_mmds ------------------------------ */
pub struct GetMmds {
    ops: GetMmdsOps,
    res: Mutex<GetMmdsRes>,
}

impl Event<GetMmdsOps, GetMmdsRes> for GetMmds {
    fn get_ops(&self) -> &GetMmdsOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut GetMmdsOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut GetMmdsRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: GetMmdsRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl GetMmds {
    pub fn new() -> Self {
        Self {
            ops: GetMmdsOps::new(),
            res: Mutex::new(GetMmdsRes::blank()),
        }
    }
}

/*------------------------------ load_snapshot ------------------------------ */
pub struct LoadSnapshot {
    ops: LoadSnapshotOps,
    res: Mutex<LoadSnapshotRes>,
}

impl Event<LoadSnapshotOps, LoadSnapshotRes> for LoadSnapshot {
    fn get_ops(&self) -> &LoadSnapshotOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut LoadSnapshotOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut LoadSnapshotRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: LoadSnapshotRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl LoadSnapshot {
    pub fn new(data: SnapshotLoadParams) -> Self {
        Self {
            ops: LoadSnapshotOps::new(data),
            res: Mutex::new(LoadSnapshotRes::blank()),
        }
    }
}

/*------------------------------ patch_balloon_stats_interval ------------------------------ */
pub struct PatchBalloonStatsInterval {
    ops: PatchBalloonStatsIntervalOps,
    res: Mutex<PatchBalloonStatsIntervalRes>,
}

impl Event<PatchBalloonStatsIntervalOps, PatchBalloonStatsIntervalRes>
    for PatchBalloonStatsInterval
{
    fn get_ops(&self) -> &PatchBalloonStatsIntervalOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut PatchBalloonStatsIntervalOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut PatchBalloonStatsIntervalRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: PatchBalloonStatsIntervalRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl PatchBalloonStatsInterval {
    pub fn new(data: BalloonStatsUpdate) -> Self {
        Self {
            ops: PatchBalloonStatsIntervalOps::new(data),
            res: Mutex::new(PatchBalloonStatsIntervalRes::blank()),
        }
    }
}

/*------------------------------ patch_balloon ------------------------------ */
pub struct PatchBalloon {
    ops: PatchBalloonOps,
    res: Mutex<PatchBalloonRes>,
}

impl Event<PatchBalloonOps, PatchBalloonRes> for PatchBalloon {
    fn get_ops(&self) -> &PatchBalloonOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut PatchBalloonOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut PatchBalloonRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: PatchBalloonRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl PatchBalloon {
    pub fn new(data: BalloonUpdate) -> Self {
        Self {
            ops: PatchBalloonOps::new(data),
            res: Mutex::new(PatchBalloonRes::blank()),
        }
    }
}

/*------------------------------ patch_guest_drive_by_id ------------------------------ */
pub struct PatchGuestDriveById {
    ops: PatchGuestDriveByIdOps,
    res: Mutex<PatchGuestDriveByIdRes>,
}

impl Event<PatchGuestDriveByIdOps, PatchGuestDriveByIdRes> for PatchGuestDriveById {
    fn get_ops(&self) -> &PatchGuestDriveByIdOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut PatchGuestDriveByIdOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut PatchGuestDriveByIdRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: PatchGuestDriveByIdRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

/*------------------------------ patch_guest_network_interface_by_id ------------------------------ */
pub struct PatchGuestNetworkInterfaceById {
    ops: PatchGuestNetworkInterfaceByIdOps,
    res: Mutex<PatchGuestNetworkInterfaceByIdRes>,
}

impl Event<PatchGuestNetworkInterfaceByIdOps, PatchGuestNetworkInterfaceByIdRes>
    for PatchGuestNetworkInterfaceById
{
    fn get_ops(&self) -> &PatchGuestNetworkInterfaceByIdOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut PatchGuestNetworkInterfaceByIdOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut PatchGuestNetworkInterfaceByIdRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: PatchGuestNetworkInterfaceByIdRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl PatchGuestNetworkInterfaceById {
    pub fn new(data: PartialNetworkInterface) -> Self {
        Self {
            ops: PatchGuestNetworkInterfaceByIdOps::new(data),
            res: Mutex::new(PatchGuestNetworkInterfaceByIdRes::blank()),
        }
    }
}

/*------------------------------ patch_machine_configuration ------------------------------ */
pub struct PatchMachineConfiguration {
    ops: PatchMachineConfigurationOps,
    res: Mutex<PatchMachineConfigurationRes>,
}

impl Event<PatchMachineConfigurationOps, PatchMachineConfigurationRes>
    for PatchMachineConfiguration
{
    fn get_ops(&self) -> &PatchMachineConfigurationOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut PatchMachineConfigurationOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut PatchMachineConfigurationRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: PatchMachineConfigurationRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl PatchMachineConfiguration {
    pub fn new(data: MachineConfiguration) -> Self {
        Self {
            ops: PatchMachineConfigurationOps::new(data),
            res: Mutex::new(PatchMachineConfigurationRes::blank()),
        }
    }
}

/*------------------------------ patch_mmds ------------------------------ */
pub struct PatchMmds {
    ops: PatchMmdsOps,
    res: Mutex<PatchMmdsRes>,
}

impl Event<PatchMmdsOps, PatchMmdsRes> for PatchMmds {
    fn get_ops(&self) -> &PatchMmdsOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut PatchMmdsOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut PatchMmdsRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: PatchMmdsRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl PatchMmds {
    pub fn new(data: MmdsContentsObject) -> Self {
        Self {
            ops: PatchMmdsOps::new(data),
            res: Mutex::new(PatchMmdsRes::blank()),
        }
    }
}

/*------------------------------ patch_vm ------------------------------ */
pub struct PatchVm {
    ops: PatchVmOps,
    res: Mutex<PatchVmRes>,
}

impl Event<PatchVmOps, PatchVmRes> for PatchVm {
    fn get_ops(&self) -> &PatchVmOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut PatchVmOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut PatchVmRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: PatchVmRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl PatchVm {
    pub fn new(data: Vm) -> Self {
        Self {
            ops: PatchVmOps::new(data),
            res: Mutex::new(PatchVmRes::blank()),
        }
    }
}

/*------------------------------ put_balloon ------------------------------ */
pub struct PutBalloon {
    ops: PutBalloonOps,
    res: Mutex<PutBalloonRes>,
}

impl Event<PutBalloonOps, PutBalloonRes> for PutBalloon {
    fn get_ops(&self) -> &PutBalloonOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut PutBalloonOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut PutBalloonRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: PutBalloonRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl PutBalloon {
    pub fn new(data: Balloon) -> Self {
        Self {
            ops: PutBalloonOps::new(data),
            res: Mutex::new(PutBalloonRes::blank()),
        }
    }
}

/*------------------------------ put_cpu_configuration ------------------------------ */
pub struct PutCpuConfiguration {
    ops: PutCpuConfigurationOps,
    res: Mutex<PutCpuConfigurationRes>,
}

impl Event<PutCpuConfigurationOps, PutCpuConfigurationRes> for PutCpuConfiguration {
    fn get_ops(&self) -> &PutCpuConfigurationOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut PutCpuConfigurationOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut PutCpuConfigurationRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: PutCpuConfigurationRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl PutCpuConfiguration {
    pub fn new(data: CPUConfig) -> Self {
        Self {
            ops: PutCpuConfigurationOps::new(data),
            res: Mutex::new(PutCpuConfigurationRes::blank()),
        }
    }
}

/*------------------------------ put_entropy ------------------------------ */
pub struct PutEntropy {
    ops: PutEntropyOps,
    res: Mutex<PutEntropyRes>,
}

impl Event<PutEntropyOps, PutEntropyRes> for PutEntropy {
    fn get_ops(&self) -> &PutEntropyOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut PutEntropyOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut PutEntropyRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: PutEntropyRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl PutEntropy {
    pub fn new(data: EntropyDevice) -> Self {
        Self {
            ops: PutEntropyOps::new(data),
            res: Mutex::new(PutEntropyRes::blank()),
        }
    }
}

/*------------------------------ put_guest_boot_source ------------------------------ */
pub struct PutGuestBootSource {
    ops: PutGuestBootSourceOps,
    res: Mutex<PutGuestBootSourceRes>,
}

impl Event<PutGuestBootSourceOps, PutGuestBootSourceRes> for PutGuestBootSource {
    fn get_ops(&self) -> &PutGuestBootSourceOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut PutGuestBootSourceOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut PutGuestBootSourceRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: PutGuestBootSourceRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl PutGuestBootSource {
    pub fn new(data: BootSource) -> Self {
        Self {
            ops: PutGuestBootSourceOps::new(data),
            res: Mutex::new(PutGuestBootSourceRes::blank()),
        }
    }
}

/*------------------------------ put_guest_drive_by_id ------------------------------ */
pub struct PutGuestDriveById {
    ops: PutGuestDriveByIdOps,
    res: Mutex<PutGuestDriveByIdRes>,
}

impl Event<PutGuestDriveByIdOps, PutGuestDriveByIdRes> for PutGuestDriveById {
    fn get_ops(&self) -> &PutGuestDriveByIdOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut PutGuestDriveByIdOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut PutGuestDriveByIdRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: PutGuestDriveByIdRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl PutGuestDriveById {
    pub fn new(data: Drive) -> Self {
        Self {
            ops: PutGuestDriveByIdOps::new(data),
            res: Mutex::new(PutGuestDriveByIdRes::blank()),
        }
    }
}

/*------------------------------ put_guest_network_interface_by_id ------------------------------ */
pub struct PutGuestNetworkInterfaceById {
    ops: PutGuestNetworkInterfaceByIdOps,
    res: Mutex<PutGuestNetworkInterfaceByIdRes>,
}

impl Event<PutGuestNetworkInterfaceByIdOps, PutGuestNetworkInterfaceByIdRes>
    for PutGuestNetworkInterfaceById
{
    fn get_ops(&self) -> &PutGuestNetworkInterfaceByIdOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut PutGuestNetworkInterfaceByIdOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut PutGuestNetworkInterfaceByIdRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: PutGuestNetworkInterfaceByIdRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl PutGuestNetworkInterfaceById {
    pub fn new(data: NetworkInterface) -> Self {
        Self {
            ops: PutGuestNetworkInterfaceByIdOps::new(data),
            res: Mutex::new(PutGuestNetworkInterfaceByIdRes::blank()),
        }
    }
}

/*------------------------------ put_guest_vsock ------------------------------ */
pub struct PutGuestVsock {
    ops: PutGuestVsockOps,
    res: Mutex<PutGuestVsockRes>,
}

impl Event<PutGuestVsockOps, PutGuestVsockRes> for PutGuestVsock {
    fn get_ops(&self) -> &PutGuestVsockOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut PutGuestVsockOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut PutGuestVsockRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: PutGuestVsockRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl PutGuestVsock {
    pub fn new(data: Vsock) -> Self {
        Self {
            ops: PutGuestVsockOps::new(data),
            res: Mutex::new(PutGuestVsockRes::blank()),
        }
    }
}

/*------------------------------ put_logger ------------------------------ */
pub struct PutLogger {
    ops: PutLoggerOps,
    res: Mutex<PutLoggerRes>,
}

impl Event<PutLoggerOps, PutLoggerRes> for PutLogger {
    fn get_ops(&self) -> &PutLoggerOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut PutLoggerOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut PutLoggerRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: PutLoggerRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl PutLogger {
    pub fn new(data: Logger) -> Self {
        Self {
            ops: PutLoggerOps::new(data),
            res: Mutex::new(PutLoggerRes::blank()),
        }
    }
}

/*------------------------------ put_machine_configuration ------------------------------ */
pub struct PutMachineConfiguration {
    ops: PutMachineConfigurationOps,
    res: Mutex<PutMachineConfigurationRes>,
}

impl Event<PutMachineConfigurationOps, PutMachineConfigurationRes> for PutMachineConfiguration {
    fn get_ops(&self) -> &PutMachineConfigurationOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut PutMachineConfigurationOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut PutMachineConfigurationRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: PutMachineConfigurationRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl PutMachineConfiguration {
    pub fn new(data: MachineConfiguration) -> Self {
        Self {
            ops: PutMachineConfigurationOps::new(data),
            res: Mutex::new(PutMachineConfigurationRes::blank()),
        }
    }
}

/*------------------------------ put_metrics ------------------------------ */
pub struct PutMetrics {
    ops: PutMetricsOps,
    res: Mutex<PutMetricsRes>,
}

impl Event<PutMetricsOps, PutMetricsRes> for PutMetrics {
    fn get_ops(&self) -> &PutMetricsOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut PutMetricsOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut PutMetricsRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: PutMetricsRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl PutMetrics {
    pub fn new(data: Metrics) -> Self {
        Self {
            ops: PutMetricsOps::new(data),
            res: Mutex::new(PutMetricsRes::blank()),
        }
    }
}

/*------------------------------ put_mmds_config ------------------------------ */
pub struct PutMmdsConfig {
    ops: PutMmdsConfigOps,
    res: Mutex<PutMmdsConfigRes>,
}

impl Event<PutMmdsConfigOps, PutMmdsConfigRes> for PutMmdsConfig {
    fn get_ops(&self) -> &PutMmdsConfigOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut PutMmdsConfigOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut PutMmdsConfigRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: PutMmdsConfigRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl PutMmdsConfig {
    pub fn new(data: MmdsConfig) -> Self {
        Self {
            ops: PutMmdsConfigOps::new(data),
            res: Mutex::new(PutMmdsConfigRes::blank()),
        }
    }
}

/*------------------------------ put_mmds ------------------------------ */
pub struct PutMmds {
    ops: PutMmdsOps,
    res: Mutex<PutMmdsRes>,
}

impl Event<PutMmdsOps, PutMmdsRes> for PutMmds {
    fn get_ops(&self) -> &PutMmdsOps {
        &self.ops
    }

    fn get_ops_mut(&mut self) -> &mut PutMmdsOps {
        &mut self.ops
    }

    fn get_res_mut(&mut self) -> RtckResult<&mut PutMmdsRes> {
        Ok(self.res.get_mut()?)
    }

    fn set_res(&self, res: PutMmdsRes) -> RtckResult<()> {
        *self.res.lock()? = res;
        Ok(())
    }
}

impl PutMmds {
    pub fn new(data: MmdsContentsObject) -> Self {
        Self {
            ops: PutMmdsOps::new(data),
            res: Mutex::new(PutMmdsRes::blank()),
        }
    }
}
