use serde::{Deserialize, Serialize};

/// Balloon device descriptor.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BalloonUpdate {
    /// Target balloon size in MiB.
    #[serde(rename = "amount_mib")]
    pub amount_mib: i64,
}
