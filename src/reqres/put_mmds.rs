#[repr(transparent)]
pub struct PutMmdsRequest(MmdsContentsObject);

#[repr(transparent)]
pub struct PutMmdsResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct PutMmds(pub PutMmdsRequest);

impl_all_firecracker_traits!(
    PutMmds,
    "PUT",
    "/mmds",
    MmdsContentsObject,
    Empty,
    InternalError
);
