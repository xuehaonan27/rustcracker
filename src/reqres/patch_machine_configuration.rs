#[repr(transparent)]
pub struct PatchMachineConfigurationRequest(MachineConfiguration);

#[repr(transparent)]
pub struct PatchMachineConfigurationResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct PatchMachineConfiguration(pub PatchMachineConfigurationRequest);

impl_all_firecracker_traits!(
    PatchMachineConfiguration,
    "PATCH",
    "/machine-config",
    MachineConfiguration,
    Empty,
    InternalError
);
