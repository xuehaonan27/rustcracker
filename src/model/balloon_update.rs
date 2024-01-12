use serde::{Serialize, Deserialize};

use crate::utils::Json;

/// Balloon device descriptor.
/// # Example
/// 
/// ```
/// // This piece of code will give you a balloon device
/// // update structure with new memory allocation set to
/// // 2048 MiB, which is used for updating the balloon 
/// // device. Before or after machine startup.
/// use Rustcracker::model::balloon_stats_update::BalloonStatsUpdate;
/// let balloon_stats_update = BalloonStatsUpdate::new(10);
/// ```
#[derive(Serialize, Deserialize)]
pub struct BalloonUpdate {
    amount_mib: u64,
}
impl<'a> Json<'a> for BalloonUpdate {
    type Item = BalloonUpdate;
}