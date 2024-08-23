use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
        Arc,
    },
};

use crossbeam::queue::ArrayQueue;
use tokio::time::timeout;

use crate::{config::HypervisorConfig, hypervisor::Hypervisor, RtckError, RtckResult};

use super::{options::PoolOptions, utils::{AsyncSemaphore, AsyncSemaphoreReleaser}, PoolError};

pub struct PoolInner {
    pub(super) hypervisors: ArrayQueue<Hypervisor>,
    pub(super) semaphore: AsyncSemaphore,
    pub(super) size: AtomicU32,
    pub(super) num_idle: AtomicUsize,
    is_closed: AtomicBool,
    pub(super) options: PoolOptions,
}

impl PoolInner {
    pub(super) fn new_arc(options: PoolOptions) -> Arc<Self> {
        let capacity = options.max_connections as usize;

        let pool = Self {
            hypervisors: ArrayQueue::new(capacity),
            semaphore: AsyncSemaphore::new(options.fair, capacity),
            size: AtomicU32::new(0),
            num_idle: AtomicUsize::new(0),
            is_closed: AtomicBool::new(false),
        
            options,
        };

        let pool = Arc::new(pool);

        pool
    }

    pub(super) fn size(&self) -> u32 {
        self.size.load(Ordering::Acquire)
    }

    pub(super) fn num_idle(&self) -> usize {
        // We don't use `self.idle_conns.len()` as it waits for the internal
        // head and tail pointers to stop changing for a moment before calculating the length,
        // which may take a long time at high levels of churn.
        //
        // By maintaining our own atomic count, we avoid that issue entirely.
        self.num_idle.load(Ordering::Acquire)
    }

    pub(super) fn is_closed(&self) -> bool {
        self.is_closed.load(Ordering::Acquire)
    }

    fn mark_closed(&self) {
        self.is_closed.store(true, Ordering::Release);
    }

    pub(super) fn close<'a>(self: &'a Arc<Self>) -> impl Future<Output = ()> + 'a {
        self.mark_closed();

        async move {
            for permits in 1..=self.options.max_connections {
                // Close any currently idle connections in the pool.
                while let Some(mut hypervisor) = self.hypervisors.pop() {
                    // FIXME: check logit here
                    let _ = hypervisor.stop().await;
                    let _ = hypervisor.delete().await;
                }

                if self.size() == 0 {
                    break;
                }

                // Wait for all permits to be released.
                let _permits = self.semaphore.acquire(permits).await;
            }
        }
    }

    /// Pull a permit from `self.semaphore`
    async fn acquire_permit<'a> (self: &'a Arc<Self>) -> AsyncSemaphoreReleaser<'a> {
        self.semaphore.acquire(1).await
    }

    pub(super) async fn acquire(self: &Arc<Self>, config: &HypervisorConfig) -> RtckResult<()> {
        if self.is_closed() {
            return Err(RtckError::Pool(PoolError::PoolClosed));
        }

        let mut finished = false;

        let _ = timeout(self.options.launch_timeout.unwrap(), async {
            let permit = self.acquire_permit().await;
            match Hypervisor::new(config).await {
                Ok(hypervisor) => match self.hypervisors.push(hypervisor) {
                    Ok(_) => {
                        return;
                    }
                    Err(h) => {

                    }
                }
                Err(e) => {

                }
            }
        }).await.map_err(|_| PoolError::PoolTimeOut)?;

       
        todo!()
    }
}
