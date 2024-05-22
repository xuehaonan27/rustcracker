use either::Either;

use crate::{
    command::Command,
    models::{error::InternalError, vsock::Vsock},
    ser::Empty,
};

use super::{Operation, Response};

pub struct PutGuestVsockOps {
    data: Vsock,
}

impl PutGuestVsockOps {
    pub fn new(data: Vsock) -> Self {
        Self { data }
    }
}

impl Operation for PutGuestVsockOps {
    fn encode(&self) -> crate::command::Command {
        Command {
            method: crate::micro_http::HttpMethod::PUT,
            url: "/vsock".into(),
            data: Box::new(self.data.clone()),
        }
    }
}

pub struct PutGuestVsockRes {
    data: Either<Empty, InternalError>,
}

impl PutGuestVsockRes {
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

impl Response for PutGuestVsockRes {
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