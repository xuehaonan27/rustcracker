use either::Either;

use crate::{
    command::Command,
    models::{error::InternalError, instance_info::InstanceInfo},
    ser::Empty,
};

use super::{Operation, Response};

pub struct DescribeInstanceOps {
    data: Empty,
}

impl DescribeInstanceOps {
    pub fn new() -> Self {
        Self { data: Empty {} }
    }
}

impl Operation for DescribeInstanceOps {
    fn encode(&self) -> crate::command::Command {
        Command {
            method: crate::micro_http::HttpMethod::GET,
            url: "/".into(),
            data: Box::new(self.data),
        }
    }
}

pub struct DescribeInstanceRes {
    data: Either<InstanceInfo, InternalError>,
}

impl DescribeInstanceRes {
    pub fn is_succ(&self) -> bool {
        self.data.is_left()
    }

    pub fn is_err(&self) -> bool {
        self.data.is_right()
    }

    pub fn succ(self) -> InstanceInfo {
        self.data.left().expect("Response is InternalError")
    }

    pub fn err(self) -> InternalError {
        self.data.right().expect("Response is successful")
    }
}

impl Response for DescribeInstanceRes {
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
