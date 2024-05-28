use either::Either;

use crate::{
    command::Command,
    micro_http::Method,
    models::{error::InternalError, machine_configuration::MachineConfiguration},
    ser::Empty,
};

use super::{RtckOperation, RtckResponse};

pub struct GetMachineConfigurationOps {
    data: Empty,
}

impl GetMachineConfigurationOps {
    pub fn new() -> Self {
        Self { data: Empty {} }
    }
}

impl RtckOperation for GetMachineConfigurationOps {
    fn encode(&self) -> crate::command::Command {
        Command {
            method: Method::Get,
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

    pub fn succ(&self) -> &MachineConfiguration {
        self.data.as_ref().left().expect("Response is InternalError")
    }

    pub fn err(&self) -> &InternalError {
        self.data.as_ref().right().expect("Response is successful")
    }
}

impl RtckResponse for GetMachineConfigurationRes {
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
