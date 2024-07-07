#[repr(transparent)]
pub struct CreateSnapshotRequest(SnapshotCreateParams);

#[repr(transparent)]
pub struct CreateSnapshotResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct CreateSnapshot(pub CreateSnapshotRequest);

impl_all_firecracker_traits!(
    CreateSnapshot,
    "PUT",
    "/snapshot/create",
    SnapshotCreateParams,
    Empty,
    InternalError
);
