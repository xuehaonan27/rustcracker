use event_listener::EventListener;
use inner::PoolInner;
use std::future::Future;
use std::sync::Arc;

use crate::config::HypervisorConfig;
use crate::RtckResult;

pub mod inner;
pub mod options;
pub mod utils;

pub struct Pool(pub(crate) Arc<PoolInner>);

impl Clone for Pool {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl Pool {
    pub async fn spawn(&self, config: &HypervisorConfig) -> RtckResult<Self> {
        todo!()
    }
    pub async fn reap(&self, id: &String) -> RtckResult<()> {
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("Pool closed")]
    PoolClosed,
    #[error("Pool spawn instance timeout")]
    PoolTimeOut,
}
