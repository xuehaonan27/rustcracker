use serde::{Serialize, Deserialize};

use crate::utils::Json;

/// Describes the balloon device statistics.
/// # Example
/// 
/// ```
/// // This piece of code will give you a balloon statistics
/// // update structure with new statistics polling interval 
/// // set to 10 seconds which is used for updating the balloon 
/// // device. Before or after machine startup.
/// use Rustcracker::model::balloon_stats_update::BalloonStatsUpdate;
/// let balloon_stats_update = BalloonStatsUpdate::new(10);
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BalloonStatsUpdate {
    /// Interval in seconds between refreshing statistics.
    #[serde(rename = "stats_polling_interval_s")]
    pub stats_polling_interval_s: i64,
}
impl<'a> Json<'a> for BalloonStatsUpdate {
    type Item = BalloonStatsUpdate;
}

impl BalloonStatsUpdate {
    /// Create a balloon statistics updating config with
    /// new statistics polling interval set to `sec` seconds.
    pub fn new(sec: i64) -> Self {
        Self { stats_polling_interval_s: sec }
    }
}