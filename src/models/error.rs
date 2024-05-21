use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InternalError {
    /// A description of the error condition
    /// readOnly: true
    #[serde(rename = "fault_message")]
    pub fault_message: String,
}
