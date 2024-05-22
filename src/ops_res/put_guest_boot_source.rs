use either::Either;

use crate::{
    command::Command,
    models::{boot_source::BootSource, error::InternalError},
    ser::Empty,
};

use super::{Operation, Response};

pub struct PutGuestBootSourceOps {
    data: BootSource,
}

impl PutGuestBootSourceOps {
    pub fn new(data: BootSource) -> Self {
        Self { data }
    }
}

impl Operation for PutGuestBootSourceOps {
    fn encode(&self) -> crate::command::Command {
        Command {
            method: crate::micro_http::HttpMethod::PUT,
            url: "/boot-source".into(),
            data: Box::new(self.data.clone()),
        }
    }
}

pub struct PutGuestBootSourceRes {
    data: Either<Empty, InternalError>,
}

impl PutGuestBootSourceRes {
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

impl Response for PutGuestBootSourceRes {
    type Data = Self;
    fn decode(res: &crate::micro_http::HttpResponse) -> crate::RtckResult<Self::Data> {
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
