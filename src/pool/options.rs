use std::time::Duration;

pub struct PoolOptions {
    pub(super) max_connections: u32,
    pub(super) max_lifetime: Option<Duration>,
    pub(super) launch_timeout: Option<Duration>,
    pub(super) fair: bool,
}

impl PoolOptions {
    pub fn new() -> Self {
        Self {
            max_connections: 10,
            max_lifetime: None,
            launch_timeout: Some(Duration::from_secs(60)),
            fair: true,
        }
    }

    pub fn max_connections(mut self, max: u32) -> Self {
        self.max_connections = max;
        self
    }

    pub fn get_max_connections(&self) -> u32 {
        self.max_connections
    }

    pub fn max_lifetime(mut self, lifetime: impl Into<Option<Duration>>) -> Self {
        self.max_lifetime = lifetime.into();
        self
    }

    /// Get the maximum lifetime of individual hypervisor.
    pub fn get_max_lifetime(&self) -> Option<Duration> {
        self.max_lifetime
    }
}
