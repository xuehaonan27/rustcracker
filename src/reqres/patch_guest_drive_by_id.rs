#[repr(transparent)]
pub struct PatchGuestDriveByIDRequest(PartialDrive);

#[repr(transparent)]
pub struct PatchGuestDriveByIDResponse(pub Either<Empty, InternalError>);

#[repr(transparent)]
pub struct PatchGuestDriveByID(pub PatchGuestDriveByIDRequest);

impl_all_firecracker_traits!(
    PatchGuestDriveByID,
    "PATCH",
    "/drives",
    PartialDrive,
    drive_id,
    Empty,
    InternalError
);
