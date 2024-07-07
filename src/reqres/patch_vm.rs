#[repr(transparent)]
pub struct PatchVmRequest(Vm);

#[repr(transparent)]
pub struct PatchVmResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct PatchVm(pub PatchVmRequest);

impl_all_firecracker_traits!(PatchVm, "PATCH", "/vm", Vm, Empty, InternalError);
