use serde::{Serialize, Deserialize};

use crate::utils::Json;

use super::rate_limiter::RateLimiter;

#[derive(Serialize, Deserialize)]
pub struct EntropyDevice {
    rate_limiter: Option<RateLimiter>,
}

impl<'a> Json<'a> for EntropyDevice {
    type Item = EntropyDevice;
}