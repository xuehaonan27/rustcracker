use serde::{Deserialize, Serialize};

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

pub const VM_STATE_PAUSED: Vm = Vm {
    state: State::Paused,
};
pub const VM_STATE_RESUMED: Vm = Vm {
    state: State::Resumed,
};
