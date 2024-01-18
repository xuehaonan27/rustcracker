use serde::{Serialize, Deserialize};

use crate::utils::Json;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InternalError {
    /// A description of the error condition
    /// readOnly: true
    #[serde(rename = "fault_message")]
    pub fault_message: String,
}

impl<'a> Json<'a> for InternalError {
    type Item = InternalError;
}