use crate::command::Command;
use crate::micro_http::HttpResponse;
use crate::RtckResult;

pub trait Operation {
    fn encode(&self) -> Command;
}

pub trait Response {
    type Data;
    fn decode(res: &HttpResponse) -> RtckResult<Self::Data>;
}

pub mod get_machine_configuration;
pub mod put_machine_configuration;
