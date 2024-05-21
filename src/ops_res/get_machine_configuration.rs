use crate::{
    command::Command, micro_http::HttpMethod, models::machine_configuration::MachineConfiguration,
    ser::Empty,
};

use super::{Operation, Response};

pub struct GetMachineConfigurationOperation {
    data: Empty,
}

impl GetMachineConfigurationOperation {
    pub fn new() -> Self {
        Self { data: Empty {} }
    }
}

impl Operation for GetMachineConfigurationOperation {
    fn encode(&self) -> crate::command::Command {
        Command {
            method: HttpMethod::GET,
            url: "/machine-config".into(),
            data: Box::new(self.data),
        }
    }
}

pub struct GetMachineConfigurationResponse {
    data: MachineConfiguration,
}

impl Response for GetMachineConfigurationResponse {
    type Data = Self;
    fn decode(res: &crate::micro_http::HttpResponse) -> crate::RtckResult<Self::Data> {
        let data: MachineConfiguration = serde_json::from_slice(res.body().as_bytes())?;
        Ok(Self { data })
    }
}
