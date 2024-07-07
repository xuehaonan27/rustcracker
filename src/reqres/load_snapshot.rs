#[repr(transparent)]
pub struct LoadSnapshotRequest(SnapshotLoadParams);

#[repr(transparent)]
pub struct LoadSnapshotResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct LoadSnapshot(pub LoadSnapshotRequest);

impl_all_firecracker_traits!(
    LoadSnapshot,
    "PUT",
    "/snapshot/load",
    SnapshotLoadParams,
    Empty,
    InternalError
);
