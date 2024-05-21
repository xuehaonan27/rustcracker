use serde::{Deserialize, Serialize};

use crate::command::Command;
use crate::micro_http::Http;
use crate::RtckResult;
use crate::{micro_http::HttpMethod, ser::Serde};

use crate::models::machine_configuration::MachineConfiguration;

pub(crate) trait Operation {
    fn encode(&self) -> Command;
}

pub(crate) trait Response {
    type Data;
    fn decode<S: AsRef<str>>(res: S) -> RtckResult<Self::Data>;
}

pub mod get_machine_configuration;