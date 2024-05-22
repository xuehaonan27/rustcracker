use either::Either;

use crate::{
    command::Command,
    micro_http::HttpMethod,
    models::{error::InternalError, machine_configuration::MachineConfiguration},
    ser::Empty,
};

use super::{Operation, Response};

pub struct GetMachineConfigurationOps {
    data: Empty,
}

impl GetMachineConfigurationOps {
    pub fn new() -> Self {
        Self { data: Empty {} }
    }
}

impl Operation for GetMachineConfigurationOps {
    fn encode(&self) -> crate::command::Command {
        Command {
            method: HttpMethod::GET,
            url: "/machine-config".into(),
            data: Box::new(self.data),
        }
    }
}

pub struct GetMachineConfigurationRes {
    data: Either<MachineConfiguration, InternalError>,
}

impl GetMachineConfigurationRes {
    pub fn is_succ(&self) -> bool {
        self.data.is_left()
    }

    pub fn is_err(&self) -> bool {
        self.data.is_right()
    }

    pub fn succ(self) -> MachineConfiguration {
        self.data.left().expect("Response is InternalError")
    }

    pub fn err(self) -> InternalError {
        self.data.right().expect("Response is successful")
    }
}

impl Response for GetMachineConfigurationRes {
    type Data = Self;
    fn new() -> Self {
        Self {
            data: Either::Right(InternalError {
                fault_message: "Initial response body".into(),
            }),
        }
    }

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
