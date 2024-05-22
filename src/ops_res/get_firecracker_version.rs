use either::Either;

use crate::{
    command::Command,
    models::{error::InternalError, firecracker_version::FirecrackerVersion},
    ser::Empty,
};

use super::{Operation, Response};

pub struct GetFirecrackerVersionOps {
    data: Empty,
}

impl GetFirecrackerVersionOps {
    pub fn new() -> Self {
        Self { data: Empty {} }
    }
}

impl Operation for GetFirecrackerVersionOps {
    fn encode(&self) -> crate::command::Command {
        Command {
            method: crate::micro_http::HttpMethod::GET,
            url: "/version".into(),
            data: Box::new(self.data),
        }
    }
}

pub struct GetFirecrackerVersionRes {
    data: Either<FirecrackerVersion, InternalError>,
}

impl GetFirecrackerVersionRes {
    pub fn is_succ(&self) -> bool {
        self.data.is_left()
    }

    pub fn is_err(&self) -> bool {
        self.data.is_right()
    }

    pub fn succ(self) -> FirecrackerVersion {
        self.data.left().expect("Response is InternalError")
    }

    pub fn err(self) -> InternalError {
        self.data.right().expect("Response is successful")
    }
}

impl Response for GetFirecrackerVersionRes {
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
