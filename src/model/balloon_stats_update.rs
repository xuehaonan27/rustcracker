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
#[derive(Serialize, Deserialize)]
pub struct BalloonStatsUpdate {
    stats_polling_interval_s: isize,
}
impl<'a> Json<'a> for BalloonStatsUpdate {
    type Item = BalloonStatsUpdate;
}

impl BalloonStatsUpdate {
    /// Create a balloon statistics updating config with
    /// new statistics polling interval set to `sec` seconds.
    pub fn new(sec: isize) -> Self {
        Self { stats_polling_interval_s: sec }
    }
}