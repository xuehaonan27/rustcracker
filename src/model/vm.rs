use serde::{Serialize, Deserialize};

use crate::utils::Json;

// VM Defines the microVM running state.
// It is especially useful in the snapshotting context.
#[derive(Serialize, Deserialize)]
pub struct Vm {
    // state
	// Required: true
	// Enum: [Paused Resumed]
    state: State,
}

#[derive(Serialize, Deserialize)]
enum State {
    Paused,
    Resumed,
}

impl<'a> Json<'a> for Vm {
    type Item = Vm;
}