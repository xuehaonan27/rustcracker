#[repr(transparent)]
pub struct GetFirecrackerVersionRequest;

#[repr(transparent)]
pub struct GetFirecrackerVersionResponse(pub Either<FirecrackerVersion, InternalError>);

#[repr(transparent)]
pub struct GetFirecrackerVersion(pub GetFirecrackerVersionRequest);

impl_all_firecracker_traits!(
    GetFirecrackerVersion,
    "GET",
    "/version",
    FirecrackerVersion,
    InternalError
);
