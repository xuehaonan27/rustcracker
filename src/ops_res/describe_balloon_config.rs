use either::Either;

use crate::{command::Command, models::{balloon::Balloon, error::InternalError}, ser::Empty};

use super::{Operation, Response};

pub struct DescribeBalloonConfigOps {
    data: Empty,
}

impl DescribeBalloonConfigOps {
    pub fn new() -> Self {
        Self { data: Empty {  } }
    }
}

impl Operation for DescribeBalloonConfigOps {
    fn encode(&self) -> crate::command::Command {
        Command {
            method: crate::micro_http::HttpMethod::GET,
            url: "/balloon".into(),
            data: Box::new(self.data)
        }
    }
}

pub struct DescribeBalloonConfigRes {
    data: Either<Balloon, InternalError>,
}

impl DescribeBalloonConfigRes {
    pub fn is_succ(&self) -> bool {
        self.data.is_left()
    }

    pub fn is_err(&self) -> bool {
        self.data.is_right()
    }

    pub fn succ(self) -> Balloon {
        self.data.left().expect("Response is InternalError")
    }

    pub fn err(self) -> InternalError {
        self.data.right().expect("Response is successful")
    }
}

impl Response for DescribeBalloonConfigRes {
    type Data = Self;
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