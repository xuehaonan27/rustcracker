use either::Either;

use crate::{
    command::Command,
    models::{error::InternalError, full_vm_configuration::FullVmConfiguration},
    ser::Empty,
};

use super::{Operation, Response};

pub struct GetExportVmConfigOps {
    data: Empty,
}

impl GetExportVmConfigOps {
    pub fn new() -> Self {
        Self { data: Empty {} }
    }
}

impl Operation for GetExportVmConfigOps {
    fn encode(&self) -> crate::command::Command {
        Command {
            method: crate::micro_http::HttpMethod::GET,
            url: "/vm/config".into(),
            data: Box::new(self.data),
        }
    }
}

pub struct GetExportVmConfigRes {
    data: Either<FullVmConfiguration, InternalError>,
}

impl GetExportVmConfigRes {
    pub fn is_succ(&self) -> bool {
        self.data.is_left()
    }

    pub fn is_err(&self) -> bool {
        self.data.is_right()
    }

    pub fn succ(self) -> FullVmConfiguration {
        self.data.left().expect("Response is InternalError")
    }

    pub fn err(self) -> InternalError {
        self.data.right().expect("Response is successful")
    }
}

impl Response for GetExportVmConfigRes {
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
