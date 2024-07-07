#[repr(transparent)]
pub struct PutMmdsConfigRequest(MmdsConfig);

#[repr(transparent)]
pub struct PutMmdsConfigResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct PutMmdsConfig(pub PutMmdsConfigRequest);

impl_all_firecracker_traits!(
    PutMmdsConfig,
    "PUT",
    "/mmds/config",
    MmdsConfig,
    Empty,
    InternalError
);
