#[repr(transparent)]
pub struct DescribeBalloonConfigRequest;

#[repr(transparent)]
pub struct DescribeBalloonConfigResponse(pub Either<Balloon, InternalError>);

#[repr(transparent)]
pub struct DescribeBalloonConfig(pub DescribeBalloonConfigRequest);

impl_all_firecracker_traits!(DescribeBalloonConfig, "GET", "/", Balloon, InternalError);
