#[repr(transparent)]
pub struct PatchMmdsRequest(MmdsContentsObject);

#[repr(transparent)]
pub struct PatchMmdsResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct PatchMmds(pub PatchMmdsRequest);

impl_all_firecracker_traits!(
    PatchMmds,
    "PATCH",
    "/mmds",
    MmdsContentsObject,
    Empty,
    InternalError
);
