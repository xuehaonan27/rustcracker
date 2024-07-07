#[repr(transparent)]
pub struct PutCpuConfigurationRequest(CPUConfig);

#[repr(transparent)]
pub struct PutCpuConfigurationResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct PutCpuConfiguration(pub PutCpuConfigurationRequest);

impl_all_firecracker_traits!(
    PutCpuConfiguration,
    "PUT",
    "/cpu-config",
    CPUConfig,
    Empty,
    InternalError
);
