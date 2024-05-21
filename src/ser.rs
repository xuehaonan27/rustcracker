use crate::models::machine_configuration::MachineConfiguration;

pub(crate) trait Serde {
    fn encode(&self) -> String;

    fn decode<S: AsRef<str>>(line: &S) -> Self
    where
        Self: Sized;
}

impl Serde for MachineConfiguration {
    fn encode(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }

    fn decode<S: AsRef<str>>(line: &S) -> Self {
        serde_json::from_str(line.as_ref()).unwrap()
    }
}

#[repr(transparent)]
pub struct Empty {}

impl Serde for Empty {
    fn decode<S: AsRef<str>>(line: &S) -> Self
    where
        Self: Sized,
    {
        Empty {}
    }
    fn encode(&self) -> String {
        String::new()
    }
}
