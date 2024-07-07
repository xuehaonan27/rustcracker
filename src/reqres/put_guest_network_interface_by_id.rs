#[repr(transparent)]
pub struct PutGuestNetworkInterfaceByIDRequest(NetworkInterface);

#[repr(transparent)]
pub struct PutGuestNetworkInterfaceByIDResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct PutGuestNetworkInterfaceByID(pub PutGuestNetworkInterfaceByIDRequest);

impl_all_firecracker_traits!(
    PutGuestNetworkInterfaceByID,
    "PUT",
    "/network-interfaces",
    NetworkInterface,
    iface_id,
    Empty,
    InternalError
);
