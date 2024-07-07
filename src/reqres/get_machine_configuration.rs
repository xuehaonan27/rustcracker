#[repr(transparent)]
pub struct GetMachineConfigurationRequest;

#[repr(transparent)]
pub struct GetMachineConfigurationResponse(pub Either<MachineConfiguration, InternalError>);

#[repr(transparent)]
pub struct GetMachineConfiguration(pub GetMachineConfigurationRequest);

impl_all_firecracker_traits!(
    GetMachineConfiguration,
    "GET",
    "/machine-config",
    MachineConfiguration,
    InternalError
);
