use either::Either;

use crate::{command::Command, models::error::InternalError, ser::Empty};

use super::{Operation, Response};

pub struct GetMmdsOps {
    data: Empty,
}

impl GetMmdsOps {
    pub fn new() -> Self {
        Self { data: Empty {} }
    }
}

impl Operation for GetMmdsOps {
    fn encode(&self) -> crate::command::Command {
        Command {
            method: crate::micro_http::HttpMethod::GET,
            url: "/mmds".into(),
            data: Box::new(self.data),
        }
    }
}

pub struct GetMmdsRes {
    data: Either<String, InternalError>,
}

impl GetMmdsRes {
    pub fn is_succ(&self) -> bool {
        self.data.is_left()
    }

    pub fn is_err(&self) -> bool {
        self.data.is_right()
    }

    pub fn succ(self) -> String {
        self.data.left().expect("Response is InternalError")
    }

    pub fn err(self) -> InternalError {
        self.data.right().expect("Response is successful")
    }
}

impl Response for GetMmdsRes {
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
