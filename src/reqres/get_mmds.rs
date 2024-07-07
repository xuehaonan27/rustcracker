#[repr(transparent)]
pub struct GetMmdsRequest;

#[repr(transparent)]
pub struct GetMmdsResponse(pub Either<MmdsContentsObject, InternalError>);

#[repr(transparent)]
pub struct GetMmds(pub GetMmdsRequest);

impl_all_firecracker_traits!(
    GetMmds,
    "GET",
    "/mmds",
    MmdsContentsObject,
    InternalError
);