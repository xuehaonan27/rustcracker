use either::Either;

use crate::{
    command::Command,
    models::{error::InternalError, snapshot_load_params::SnapshotLoadParams},
    ser::Empty,
};

use super::{Operation, Response};

pub struct LoadSnapshotOps {
    data: SnapshotLoadParams,
}

impl LoadSnapshotOps {
    pub fn new(data: SnapshotLoadParams) -> Self {
        Self { data }
    }
}

impl Operation for LoadSnapshotOps {
    fn encode(&self) -> crate::command::Command {
        Command {
            method: crate::micro_http::HttpMethod::PUT,
            url: "/snapshot/load".into(),
            data: Box::new(self.data.clone()),
        }
    }
}

pub struct LoadSnapshotRes {
    data: Either<Empty, InternalError>,
}

impl LoadSnapshotRes {
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

impl Response for LoadSnapshotRes {
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
