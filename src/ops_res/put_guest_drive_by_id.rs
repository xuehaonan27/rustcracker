use either::Either;

use crate::{
    models::{drive::Drive, error::InternalError},
    ser::Empty,
};

use super::{Operation, Response};

pub struct PutGuestDriveByIdOps {
    data: Drive,
}

impl PutGuestDriveByIdOps {
    pub fn new(data: Drive) -> Self {
        Self { data }
    }
}

impl Operation for PutGuestDriveByIdOps {
    fn encode(&self) -> crate::command::Command {
        let drive_id = &self.data.drive_id;
        crate::command::Command {
            method: crate::micro_http::HttpMethod::PUT,
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

    pub fn succ(self) -> Empty {
        self.data.left().expect("Response is InternalError")
    }

    pub fn err(self) -> InternalError {
        self.data.right().expect("Response is successful")
    }
}

impl Response for PutGuestDriveByIdRes {
    type Data = Self;
    fn blank() -> Self {
        Self {
            data: Either::Right(InternalError {
                fault_message: "Rustcracker: initial empty response".into(),
            }),
        }
    }
    fn decode(res: &crate::micro_http::HttpResponse) -> crate::RtckResult<Self> {
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
