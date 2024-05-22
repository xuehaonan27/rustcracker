use either::Either;

use crate::{
    command::Command,
    models::{error::InternalError, mmds_config::MmdsContentsObject},
    ser::Empty,
};

use super::{Operation, Response};

pub struct PatchMmdsOps {
    data: MmdsContentsObject,
}

impl PatchMmdsOps {
    pub fn new(data: MmdsContentsObject) -> Self {
        Self { data }
    }
}

impl Operation for PatchMmdsOps {
    fn encode(&self) -> crate::command::Command {
        Command {
            method: crate::micro_http::HttpMethod::PATCH,
            url: "/mmds".into(),
            data: Box::new(self.data.clone()),
        }
    }
}

pub struct PatchMmdsRes {
    data: Either<Empty, InternalError>,
}

impl PatchMmdsRes {
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

impl Response for PatchMmdsRes {
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
