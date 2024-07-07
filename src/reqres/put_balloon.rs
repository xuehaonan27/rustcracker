#[repr(transparent)]
pub struct PutBalloonRequest(Balloon);

#[repr(transparent)]
pub struct PutBalloonResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct PutBalloon(pub PutBalloonRequest);

impl_all_firecracker_traits!(PutBalloon, "PUT", "/balloon", Balloon, Empty, InternalError);
