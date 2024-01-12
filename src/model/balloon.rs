/*
@File: model/balloon.rs
@Author: Mugen_Cyaegha (Xue Haonan)
*/

use crate::utils::Json;
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
/// use Rustcracker::model::balloon::Balloon;
/// 
/// let balloon = Balloon::new()
///     .with_amount_mib(256)
///     .set_deflate_on_oom(true)
///     .with_stats_polling_interval_s(10);
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Balloon {
    // Target balloon size in MiB
    // Required: true
    amount_mib: isize,

    // Whether the balloon should deflate when then guest has memory pressure
    // Required: true
    deflate_on_oom: bool,

    // Interval in seconds between refreshing statistics
    // non-zero value will enable statistics
    // Defaults to 0
    stats_polling_interval_s: Option<isize>,
}

impl<'a> Json<'a> for Balloon {
    type Item = Balloon;
}

impl Balloon {
    pub fn new() -> Self {
        Self {
            amount_mib: 0,
            deflate_on_oom: false,
            stats_polling_interval_s: None,
        }
    }
    
    /// Set target balloon size to `m` MiB.
    pub fn with_amount_mib(mut self, m: isize) -> Self {
        self.amount_mib = m;
        self
    }

    /// Whether the balloon should deflate when the guest has memory pressure.
    /// Set to `true` to enable.
    pub fn set_deflate_on_oom(mut self, b: bool) -> Self {
        self.deflate_on_oom = b;
        self
    }

    /// Set interval between refreshing statistics to `s` seconds.
    /// A non-zero value will enable the statistics. Defaults to 0.
    /// Once set to zero (non-zero), which indicates refreshing 
    /// disabled (enabled) it couldn't be changed to enabled (disabled)
    /// after boot.
    pub fn with_stats_polling_interval_s(mut self, s: isize) -> Self {
        self.stats_polling_interval_s = Some(s);
        self
    }
}
