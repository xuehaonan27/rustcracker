//! Database implementation

use uuid::Uuid;

use crate::config::MicroVMConfig;

/// Database postgres
pub struct Database {}

impl Database {
    pub async fn store_config(&self, uuid: Uuid, config: MicroVMConfig) {
        // serialize configuration
        let serialized = serde_json::to_string(&config).unwrap();
        // store to database
    }

    pub async fn retrieve_config(&self, uuid: Uuid) -> Option<MicroVMConfig> {
        // fetch configuration from database
        let serialized = "".to_string();
        // deserialize configuration
        serde_json::from_str::<MicroVMConfig>(&serialized).ok()
    }
}
