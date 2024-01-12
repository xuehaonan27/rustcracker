use serde::{Serialize, Deserialize};

use crate::utils::Json;

#[derive(Serialize, Deserialize, Clone)]
pub enum State {
    #[serde(rename = "Not started")]
    NotStarted,
    Running,
    Paused,
}

impl From<State> for String {
    fn from(value: State) -> Self {
        match value {
            State::NotStarted => "Not started".into(),
            State::Running => "Running".into(),
            State::Paused => "Paused".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InstanceInfo {
    // Application name.
	// Required: true
    app_name: String,

    // MicroVM / instance ID.
	// Required: true
    id: String,

    // The current detailed state (Not started, Running, Paused) of the Firecracker instance. This value is read-only for the control-plane.
	// Required: true
	// Enum: [Not started Running Paused]
    state: State,

    // MicroVM hypervisor build version.
	// Required: true
    vmm_version: String,
}

impl<'a> Json<'a> for InstanceInfo {
    type Item = InstanceInfo;
}

impl InstanceInfo {
    pub fn get_app_name(&self) -> String {
        self.app_name.to_owned()
    }

    pub fn get_id(&self) -> String {
        self.id.to_owned()
    }

    pub fn get_state(&self) -> State {
        self.state.to_owned()
    }

    pub fn get_vmm_version(&self) -> String {
        self.vmm_version.to_owned()
    }
}