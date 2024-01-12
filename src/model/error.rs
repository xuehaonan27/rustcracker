use serde::{Serialize, Deserialize};

use crate::utils::Json;

#[derive(Serialize, Deserialize)]
pub struct InternalError {
    fault_message: String,
}

impl<'a> Json<'a> for InternalError {
    type Item = InternalError;
}