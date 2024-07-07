#[repr(transparent)]
pub struct PutEntropyRequest(EntropyDevice);

#[repr(transparent)]
pub struct PutEntropyResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct PutEntropy(pub PutEntropyRequest);

impl_all_firecracker_traits!(
    PutEntropy,
    "PUT",
    "/entropy",
    EntropyDevice,
    Empty,
    InternalError
);
