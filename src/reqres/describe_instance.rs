#[repr(transparent)]
pub struct DescribeInstanceRequest;

#[repr(transparent)]
pub struct DescribeInstanceResponse(pub Either<InstanceInfo, InternalError>);

#[repr(transparent)]
pub struct DescribeInstance(pub DescribeInstanceRequest);

impl_all_firecracker_traits!(DescribeInstance, "GET", "/", InstanceInfo, InternalError);
