//! Database implementation

use uuid::Uuid;

use crate::machine_dev::MachineExportedConfig;

/// Database postgres
pub struct Database {}

impl Database {
    pub async fn store_config(&self, uuid: Uuid, config: MachineExportedConfig) {
        // serialize configuration
        let serialized = serde_json::to_string(&config).unwrap();
        // store to database
    }

    pub async fn retrieve_config(&self, uuid: Uuid) -> Option<MachineExportedConfig> {
        // fetch configuration from database
        let serialized = "".to_string();
        // deserialize configuration
        serde_json::from_str::<MachineExportedConfig>(&serialized).ok()
    }
}
