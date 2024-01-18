use serde::{Deserialize, Serialize};

use crate::utils::Json;

// VM Defines the microVM running state.
// It is especially useful in the snapshotting context.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Vm {
    /// state
    /// Required: true
    /// Enum: [Paused Resumed]
    pub state: State,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum State {
    #[serde(rename = "Paused")]
    Paused,
    #[serde(rename = "Resumed")]
    Resumed,
}

impl<'a> Json<'a> for Vm {
    type Item = Vm;
}

pub const VM_STATE_PAUSED: Vm = Vm {
    state: State::Paused,
};
pub const VM_STATE_RESUMED: Vm = Vm {
    state: State::Resumed,
};