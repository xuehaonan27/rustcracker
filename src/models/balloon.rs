use serde::{Deserialize, Serialize};

/// Balloon device descriptor.
///
/// # Example
///
/// ```
/// // This piece of code will give you a Balloon
/// // configuration with target size set to 256 MiB,
/// // deflating on out-of-memory enabled, statistics
/// // refreshing enabled and set to 10 seconds between
/// // two refreshing.
/// use rustcracker::model::balloon::Balloon;
///
/// let balloon = Balloon::new()
///     .with_amount_mib(256)
///     .set_deflate_on_oom(true)
///     .with_stats_polling_interval_s(10);
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Balloon {
    /// Target balloon size in MiB
    /// Required: true
    #[serde(rename = "amount_mib")]
    pub amount_mib: i64,

    /// Whether the balloon should deflate when then guest has memory pressure
    /// Required: true
    #[serde(rename = "deflate_on_oom")]
    pub deflate_on_oom: bool,

    /// Interval in seconds between refreshing statistics
    /// non-zero value will enable statistics
    /// Defaults to 0
    #[serde(
        rename = "stats_polling_interval_s",
        skip_serializing_if = "Option::is_none"
    )]
    pub stats_polling_interval_s: Option<i64>,
}
