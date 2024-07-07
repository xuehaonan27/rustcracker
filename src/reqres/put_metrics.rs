#[repr(transparent)]
pub struct PutMetricsRequest(Metrics);

#[repr(transparent)]
pub struct PutMetricsResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct PutMetrics(pub PutMetricsRequest);

impl_all_firecracker_traits!(PutMetrics, "PUT", "/metrics", Metrics, Empty, InternalError);
