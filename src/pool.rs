use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crossbeam::queue::ArrayQueue;

use crate::hypervisor::Hypervisor;

pub struct AsyncSemaphore {
    inner: tokio::sync::Semaphore,
}

impl AsyncSemaphore {
    pub fn new(fair: bool, permits: usize) -> Self {
        AsyncSemaphore {
            inner: {
                debug_assert!(fair, "Tokio only has fair permits");
                tokio::sync::Semaphore::new(permits)
            },
        }
    }

    pub fn permits(&self) -> usize {
        self.inner.available_permits()
    }

    pub async fn acquire(&self, permits: u32) -> AsyncSemaphoreReleaser<'_> {
        AsyncSemaphoreReleaser {
            inner: self
                .inner
                // Weird quirk: `tokio::sync::Semaphore` mostly uses `usize` for permit counts,
                // but `u32` for this and `try_acquire_many()`.
                .acquire_many(permits)
                .await
                .expect("BUG: we do not expose the `.close()` method"),
        }
    }

    pub fn try_acquire(&self, permits: u32) -> Option<AsyncSemaphoreReleaser<'_>> {
        Some(AsyncSemaphoreReleaser {
            inner: self.inner.try_acquire_many(permits).ok()?,
        })
    }

    pub fn release(&self, permits: usize) {
        self.inner.add_permits(permits)
    }
}

pub struct AsyncSemaphoreReleaser<'a> {
    inner: tokio::sync::SemaphorePermit<'a>,
}

impl AsyncSemaphoreReleaser<'_> {
    pub fn disarm(self) {
        self.inner.forget();
    }
}

pub struct PoolOptions {
    pub(crate) max_connections: u32,
    pub(crate) max_lifetime: Option<Duration>,
    pub(crate) launch_timeout: Option<Duration>,
}

pub struct PoolInner {
    pub(crate) hypervisors: ArrayQueue<Hypervisor>,
    pub(crate) semaphore: AsyncSemaphore,
    pub(crate) size: AtomicU32,
    pub(crate) num_empty: AtomicUsize,
    is_closed: AtomicBool,
    pub(crate) on_closed: event_listener::Event,
    pub(crate) options: PoolOptions,
}

impl PoolInner {
    pub fn new_arc(options: PoolOptions) -> Arc<Self> {
        let capacity = options.max_connections;
        todo!()
    }

    pub fn size(&self) -> u32 {
        self.size.load(Ordering::Acquire)
    }

    pub fn num_empty(&self) -> usize {
        self.num_empty.load(Ordering::Acquire)
    }

    pub fn is_closed(&self) -> bool {
        self.is_closed.load(Ordering::Acquire)
    }

    fn mark_closed(&self) {
        self.is_closed.store(true, Ordering::Release);
        self.on_closed.notify(usize::MAX);
    }

    pub(super) fn close<'a>(self: &'a Arc<Self>) -> impl Future<Output = ()> + 'a {
        self.mark_closed();

        async move {
            for permits in 1..=self.options.max_connections {
                // Close any currently idle connections in the pool.
                while let Some(mut hypervisor) = self.hypervisors.pop() {
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
}

pub struct Pool(pub(crate) Arc<PoolInner>);

impl Clone for Pool {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
