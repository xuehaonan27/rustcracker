use crate::{command::Command, micro_http::HttpMethod, models::machine_configuration::MachineConfiguration};

use super::{Operation, Response};

pub struct GetMachineConfiguration {
    data: MachineConfiguration,
}

impl Operation for GetMachineConfiguration {
    fn encode(&self) -> Command {
        Command {
            method: HttpMethod::GET,
            url: "/machine".into(),
            data: Box::new(self.data.clone()),
        }
    }
}

impl Response for GetMachineConfiguration {
    type Data = Self;

    fn decode<S: AsRef<str>>(res: S) -> crate::RtckResult<Self::Data> {
        let data: MachineConfiguration = serde_json::from_slice(res.as_ref().as_bytes())?;
        Ok(Self {data})
    }
}
