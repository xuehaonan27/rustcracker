#[repr(transparent)]
pub struct PatchGuestNetworkInterfaceByIDRequest(PartialNetworkInterface);

#[repr(transparent)]
pub struct PatchGuestNetworkInterfaceByIDResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct PatchGuestNetworkInterfaceByID(pub PatchGuestNetworkInterfaceByIDRequest);

impl_all_firecracker_traits!(
    PatchGuestNetworkInterfaceByID,
    "PATCH",
    "/network-interfaces",
    PartialNetworkInterface,
    iface_id,
    Empty,
    InternalError
);
