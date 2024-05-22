use either::Either;

use crate::{
    command::Command,
    models::{entropy_device::EntropyDevice, error::InternalError},
    ser::Empty,
};

use super::{Operation, Response};

pub struct PutEntropyOps {
    data: EntropyDevice,
}

impl PutEntropyOps {
    pub fn new(data: EntropyDevice) -> Self {
        Self { data }
    }
}

impl Operation for PutEntropyOps {
    fn encode(&self) -> crate::command::Command {
        Command {
            method: crate::micro_http::HttpMethod::PUT,
            url: "/entropy".into(),
            data: Box::new(self.data.clone()),
        }
    }
}

pub struct PutEntropyRes {
    data: Either<Empty, InternalError>,
}

impl PutEntropyRes {
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

impl Response for PutEntropyRes {
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
