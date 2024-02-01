/*
@File: model/balloon.rs
@Author: Mugen_Cyaegha (Xue Haonan)
*/

use crate::{components::machine::MachineError, utils::Json};
use log::error;
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
/// use rustfire::model::balloon::Balloon;
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
    #[serde(rename = "deflate_on_oon")]
    pub deflate_on_oom: bool,

    /// Interval in seconds between refreshing statistics
    /// non-zero value will enable statistics
    /// Defaults to 0
    #[serde(rename = "stats_polling_interval_s", skip_serializing_if = "Option::is_none")]
    pub stats_polling_interval_s: Option<i64>,
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
    pub fn with_amount_mib(mut self, m: i64) -> Self {
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
    pub fn with_stats_polling_interval_s(mut self, s: i64) -> Self {
        self.stats_polling_interval_s = Some(s);
        self
    }

    #[must_use="must validate Balloon before putting it to microVm"]
    pub fn validate(&self) -> Result<(), MachineError> {
        if self.amount_mib < 0 {
            error!(target: "Balloon::validate", "cannot assign negative amount of target memory to the Balloon");
            return Err(MachineError::Validation("cannot assign negative amount of target memory to the Balloon".to_string()))
        }

        Ok(())
    }
}
