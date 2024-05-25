use serde::{Deserialize, Serialize};

/// Describes the balloon device statistics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BalloonStatsUpdate {
    /// Interval in seconds between refreshing statistics.
    #[serde(rename = "stats_polling_interval_s")]
    pub stats_polling_interval_s: i64,
}

impl BalloonStatsUpdate {
    /// Create a balloon statistics updating config with
    /// new statistics polling interval set to `sec` seconds.
    pub fn new(sec: i64) -> Self {
        Self {
            stats_polling_interval_s: sec,
        }
    }
}
