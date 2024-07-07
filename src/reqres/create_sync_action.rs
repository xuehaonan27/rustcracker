#[repr(transparent)]
pub struct CreateSyncActionRequest(InstanceActionInfo);

#[repr(transparent)]
pub struct CreateSyncActionResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct CreateSyncAction(pub CreateSyncActionRequest);

impl_all_firecracker_traits!(
    CreateSyncAction,
    "PUT",
    "/actions",
    InstanceActionInfo,
    Empty,
    InternalError
);
