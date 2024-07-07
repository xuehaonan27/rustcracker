#[repr(transparent)]
pub struct DescribeBalloonStatsRequest;

#[repr(transparent)]
pub struct DescribeBalloonStatsResponse(pub Either<BalloonStats, InternalError>);

#[repr(transparent)]
pub struct DescribeBalloonStats(pub DescribeBalloonStatsRequest);

impl_all_firecracker_traits!(
    DescribeBalloonStats,
    "GET",
    "/balllon/statistics",
    BalloonStats,
    InternalError
);
