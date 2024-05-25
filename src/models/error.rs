use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InternalError {
    /// A description of the error condition
    /// readOnly: true
    #[serde(rename = "fault_message")]
    pub fault_message: String,
}

impl Display for InternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fault_message)
    }
}