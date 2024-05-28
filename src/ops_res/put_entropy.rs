use either::Either;

use crate::{
    command::Command,
    models::{entropy_device::EntropyDevice, error::InternalError},
    ser::Empty,
};

use super::{RtckOperation, RtckResponse};

pub struct PutEntropyOps {
    data: EntropyDevice,
}

impl PutEntropyOps {
    pub fn new(data: EntropyDevice) -> Self {
        Self { data }
    }
}

impl RtckOperation for PutEntropyOps {
    fn encode(&self) -> crate::command::Command {
        Command {
            method: crate::micro_http::Method::Put,
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

    pub fn succ(&self) -> &Empty {
        self.data.as_ref().left().expect("Response is InternalError")
    }

    pub fn err(&self) -> &InternalError {
        self.data.as_ref().right().expect("Response is successful")
    }
}

impl RtckResponse for PutEntropyRes {
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
