#[repr(transparent)]
pub struct GetExportVmConfigRequest;

#[repr(transparent)]
pub struct GetExportVmConfigResponse(pub Either<FullVmConfiguration, InternalError>);

#[repr(transparent)]
pub struct GetExportVmConfig(pub GetExportVmConfigRequest);

impl_all_firecracker_traits!(
    GetExportVmConfig,
    "GET",
    "/vm/config",
    FullVmConfiguration,
    InternalError
);
