use serde::{Serialize, Deserialize};

use crate::utils::Json;

use super::rate_limiter::RateLimiter;
// PartialNetworkInterface Defines a partial network interface structure,
// used to update the rate limiters for that interface, after microvm start.
#[derive(Serialize, Deserialize)]
pub struct PartialNetworkInterface {
    // iface id
	// Required: true
    iface_id: String,

    // rx rate limiter
    rx_rate_limiter: Option<RateLimiter>,

    // tx rate limiter
    tx_rate_limiter: Option<RateLimiter>,
}

impl<'a> Json<'a> for PartialNetworkInterface {
    type Item = PartialNetworkInterface;
}

impl PartialNetworkInterface {
    pub fn get_iface_id(&self) -> String {
        self.iface_id.to_owned()
    }
}