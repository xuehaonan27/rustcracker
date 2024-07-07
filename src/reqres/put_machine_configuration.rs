#[repr(transparent)]
pub struct PutMachineConfigurationRequest(MachineConfiguration);

#[repr(transparent)]
pub struct PutMachineConfigurationResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct PutMachineConfiguration(pub PutMachineConfigurationRequest);

impl_all_firecracker_traits!(
    PutMachineConfiguration,
    "PUT",
    "/machine-config",
    MachineConfiguration,
    Empty,
    InternalError
);
