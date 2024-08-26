pub mod sync;
pub mod tokio;
pub use sync::Hypervisor as HypervisorSync;
pub use tokio::Hypervisor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MicroVMStatus {
    None,    // no microVM running now
    Start,   // in stage of staring
    Running, // microVM running
    Paused,  // microVM paused
    Stop,    // microVM stopped
    Delete,  // microVM deleted, waiting its resources to be collected
    Failure, // microVM encountered failure
}
