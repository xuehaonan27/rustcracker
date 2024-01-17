use serde::{Serialize, Deserialize};

use crate::utils::Json;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ActionType {
    FlushMetrics,
    InstanceStart,
    SendCtrlAtlDel,
}
// impl From<ActionType> for String {
//     fn from(value: ActionType) -> Self {
//         match value {
//             ActionType::FlushMetrics => "FlushMetrics".into(),
//             ActionType::InstanceStart => "InstanceStart".into(),
//             ActionType::SendCtrlAtlDel => "SendCtrlAtlDel".into(),
//         }
//     }
// }
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstanceActionInfo {
    // Enumeration indicating what type of action is contained in the payload
	// Required: true
	// Enum: [FlushMetrics InstanceStart SendCtrlAltDel
    action_type: ActionType,
}

impl<'a> Json<'a> for InstanceActionInfo {
    type Item = InstanceActionInfo;
}

impl InstanceActionInfo {
    pub fn flush_metrics() -> Self {
        Self { action_type: ActionType::FlushMetrics }
    }

    pub fn instance_start() -> Self {
        Self { action_type: ActionType::InstanceStart }
    }

    pub fn send_ctrl_alt_del() -> Self {
        Self { action_type: ActionType::SendCtrlAtlDel }
    }
}