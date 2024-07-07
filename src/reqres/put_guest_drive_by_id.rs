#[repr(transparent)]
pub struct PutGuestDriveByIDRequest(Drive);

#[repr(transparent)]
pub struct PutGuestDriveByIDResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct PutGuestDriveByID(pub PutGuestDriveByIDRequest);

impl_all_firecracker_traits!(
    PutGuestDriveByID,
    "PUT",
    "/drives",
    Drive,
    drive_id,
    Empty,
    InternalError
);