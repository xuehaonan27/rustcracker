#[repr(transparent)]
pub struct PutGuestBootSourceRequest(BootSource);

#[repr(transparent)]
pub struct PutGuestBootSourceResponse(Either<Empty, InternalError>);

#[repr(transparent)]
pub struct PutGuestBootSource(pub PutGuestBootSourceRequest);

impl_all_firecracker_traits!(
    PutGuestBootSource,
    "PUT",
    "/boot-source",
    BootSource,
    Empty,
    InternalError
);
