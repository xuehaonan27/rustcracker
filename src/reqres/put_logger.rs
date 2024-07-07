#[repr(transparent)]
pub struct PutLoggerRequest(Logger);

#[repr(transparent)]
pub struct PutLoggerResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct PutLogger(pub PutLoggerRequest);

impl_all_firecracker_traits!(PutLogger, "PUT", "/logger", Logger, Empty, InternalError);
