use crate::{
    command::Command,
    micro_http::{HttpMethod, HttpResponse},
    models::machine_configuration::MachineConfiguration,
    ser::Empty,
};

use super::{Operation, Response};

pub struct PutMachineConfigurationOperation {
    data: MachineConfiguration,
}

impl PutMachineConfigurationOperation {
    pub fn new(data: MachineConfiguration) -> Self {
        Self { data }
    }
}

impl Operation for PutMachineConfigurationOperation {
    fn encode(&self) -> Command {
        Command {
            method: HttpMethod::PUT,
            url: "/machine-config".into(),
            data: Box::new(self.data.clone()),
        }
    }
}

pub struct PutMachineConfigurationResponse {
    data: Empty,
}

impl Response for PutMachineConfigurationResponse {
    type Data = Self;

    fn decode(res: &HttpResponse) -> crate::RtckResult<Self::Data> {
        let data: Empty = serde_json::from_slice(res.body().as_bytes())?;
        Ok(Self { data })
    }
}
