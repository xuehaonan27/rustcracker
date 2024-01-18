use serde::{Serialize, Deserialize};

use crate::utils::Json;

/// The current detailed state (Not started, Running, Paused) of the Firecracker instance.
/// This value is read-only for the control-plane.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum State {
    #[serde(rename = "Not started")]
    NotStarted,
    #[serde(rename = "Running")]
    Running,
    #[serde(rename = "Paused")]
    Paused,
}

/// Describes MicroVM instance information.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstanceInfo {
    /// Application name.
	/// Required: true
    #[serde(rename = "app_name")]
    pub app_name: String,

    /// MicroVM / instance ID.
	/// Required: true
    #[serde(rename = "id")]
    pub id: String,

    /// The current detailed state (Not started, Running, Paused) of the Firecracker instance.
    /// This value is read-only for the control-plane.
	/// Required: true
	/// Enum: [Not started Running Paused]
    #[serde(rename = "state")]
    pub state: State,

    /// MicroVM hypervisor build version.
	/// Required: true
    #[serde(rename = "vmm_version")]
    pub vmm_version: String,
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