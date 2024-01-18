use serde::{Serialize, Deserialize};

use crate::utils::Json;

use super::rate_limiter::RateLimiter;

/// Defines an entropy device.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EntropyDevice {
    #[serde(rename = "rate_limiter")]
    pub rate_limiter: Option<RateLimiter>,
}

impl<'a> Json<'a> for EntropyDevice {
    type Item = EntropyDevice;
}