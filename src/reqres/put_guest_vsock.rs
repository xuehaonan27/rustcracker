#[repr(transparent)]
pub struct PutGuestVsockRequest(Vsock);

#[repr(transparent)]
pub struct PutGuestVsockResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct PutGuestVsock(pub PutGuestVsockRequest);

impl_all_firecracker_traits!(PutGuestVsock, "PUT", "/vsock", Vsock, Empty, InternalError);
