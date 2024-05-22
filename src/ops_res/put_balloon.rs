use either::Either;

use crate::{
    command::Command,
    models::{balloon::Balloon, error::InternalError},
    ser::Empty,
};

use super::{Operation, Response};

pub struct PutBalloonOps {
    data: Balloon,
}

impl PutBalloonOps {
    pub fn new(data: Balloon) -> Self {
        Self { data }
    }
}

impl Operation for PutBalloonOps {
    fn encode(&self) -> crate::command::Command {
        Command {
            method: crate::micro_http::HttpMethod::PUT,
            url: "/balloon".into(),
            data: Box::new(self.data.clone()),
        }
    }
}

pub struct PutBalloonRes {
    data: Either<Empty, InternalError>,
}

impl PutBalloonRes {
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

impl Response for PutBalloonRes {
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
