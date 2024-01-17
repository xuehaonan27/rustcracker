use serde::{Serialize, Deserialize};
use crate::utils::Json;

use super::token_bucket::TokenBucket;
// RateLimiter Defines an IO rate limiter with independent bytes/s and ops/s limits.
// Limits are defined by configuring each of the _bandwidth_ and _ops_ token buckets.

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RateLimiter {
    pub bandwidth: Option<TokenBucket>,
    pub ops: Option<TokenBucket>,
}

impl<'a> Json<'a> for RateLimiter {
    type Item = RateLimiter;
}

impl RateLimiter {
    pub fn new(bandwidth: TokenBucket, ops: TokenBucket) -> Self {
        Self {
            bandwidth: Some(bandwidth),
            ops: Some(ops),
        }
    }
}

// RateLimiterSet represents a pair of RateLimiters (inbound and outbound)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RateLimiterSet {
    // InRateLimiter limits the incoming bytes.
    pub in_rate_limiter: Option<RateLimiter>,

    // OutRateLimiter limits the outgoing bytes.
    pub out_rate_limiter: Option<RateLimiter>,
}
