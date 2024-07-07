#[repr(transparent)]
pub struct PatchBalloonStatsIntervalRequest(BalloonStatsUpdate);

#[repr(transparent)]
pub struct PatchBalloonStatsIntervalResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct PatchBalloonStatsInterval(pub PatchBalloonStatsIntervalRequest);

impl_all_firecracker_traits!(
    PatchBalloonStatsInterval,
    "PATCH",
    "/balloon/statistics",
    BalloonStatsUpdate,
    Empty,
    InternalError
);
