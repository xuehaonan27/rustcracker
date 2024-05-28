use either::Either;

use crate::{
    models::{drive::Drive, error::InternalError},
    ser::Empty,
};

use super::{RtckOperation, RtckResponse};

pub struct PutGuestDriveByIdOps {
    data: Drive,
}

impl PutGuestDriveByIdOps {
    pub fn new(data: Drive) -> Self {
        Self { data }
    }
}

impl RtckOperation for PutGuestDriveByIdOps {
    fn encode(&self) -> crate::command::Command {
        let drive_id = &self.data.drive_id;
        crate::command::Command {
            method: crate::micro_http::Method::Put,
            url: format!("/drives/{drive_id}"),
            data: Box::new(self.data.clone()),
        }
    }
}

pub struct PutGuestDriveByIdRes {
    data: Either<Empty, InternalError>,
}

impl PutGuestDriveByIdRes {
    pub fn is_succ(&self) -> bool {
        self.data.is_left()
    }

    pub fn is_err(&self) -> bool {
        self.data.is_right()
    }

    pub fn succ(&self) -> &Empty {
        self.data.as_ref().left().expect("Response is InternalError")
    }

    pub fn err(&self) -> &InternalError {
        self.data.as_ref().right().expect("Response is successful")
    }
}

impl RtckResponse for PutGuestDriveByIdRes {
    type Data = Self;

    fn is_succ(&self) -> bool {
        self.data.is_left()
    }

    fn is_err(&self) -> bool {
        self.data.is_right()
    }

    fn blank() -> Self {
        Self {
            data: Either::Right(InternalError {
                fault_message: "Rustcracker: initial empty response".into(),
            }),
        }
    }

    fn decode(res: &crate::micro_http::Response) -> crate::RtckResult<Self> {
        if res.is_fine() {
            Ok(Self {
                data: either::Left(serde_json::from_slice(res.body().as_bytes())?),
            })
        } else {
            Ok(Self {
                data: either::Right(serde_json::from_slice(res.body().as_bytes())?),
            })
        }
    }
}
