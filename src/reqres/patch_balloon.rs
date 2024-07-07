#[repr(transparent)]
pub struct PatchBalloonRequest(BalloonUpdate);

#[repr(transparent)]
pub struct PatchBalloonResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct PatchBalloon(pub PatchBalloonRequest);

impl_all_firecracker_traits!(
    PatchBalloon,
    "PATCH",
    "/balloon",
    BalloonUpdate,
    Empty,
    InternalError
);
