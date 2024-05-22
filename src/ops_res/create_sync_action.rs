use either::Either;

use crate::{
    command::Command,
    models::{error::InternalError, instance_action_info::InstanceActionInfo},
    ser::Empty,
};

use super::{Operation, Response};

pub struct CreateSyncActionOps {
    data: InstanceActionInfo,
}

impl CreateSyncActionOps {
    pub fn new(data: InstanceActionInfo) -> Self {
        Self { data }
    }
}

impl Operation for CreateSyncActionOps {
    fn encode(&self) -> crate::command::Command {
        Command {
            method: crate::micro_http::HttpMethod::PUT,
            url: "/actions".into(),
            data: Box::new(self.data.clone()),
        }
    }
}

pub struct CreateSyncActionRes {
    data: Either<Empty, InternalError>,
}

impl CreateSyncActionRes {
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

impl Response for CreateSyncActionRes {
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
