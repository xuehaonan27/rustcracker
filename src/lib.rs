use log::*;
mod agent;
pub mod config;
mod error;
mod firecracker;
pub mod hypervisor;
mod io;
mod jailer;
pub mod models;
mod net;
pub mod options;
mod raii;
mod reqres;
mod rt;
mod sync;
pub use crate::hypervisor::Hypervisor;
pub use crate::hypervisor::HypervisorSync;
pub use crate::error::{Error, Result};

#[doc(hidden)]
pub(crate) fn handle_entry<T: Clone>(option: &Option<T>, name: &'static str) -> Result<T> {
    option.clone().ok_or_else(|| {
        let msg = format!("Missing {name} entry");
        error!("{msg}");
        Error::Config(msg)
    })
}

#[doc(hidden)]
fn handle_entry_default<T: Clone>(entry: &Option<T>, default: T) -> T {
    if entry.as_ref().is_some() {
        entry.as_ref().unwrap().clone()
    } else {
        default
    }
}

#[doc(hidden)]
fn handle_entry_ref<'a, T>(entry: &'a Option<T>, name: &'static str) -> Result<&'a T> {
    entry.as_ref().ok_or_else(|| {
        let msg = format!("Missing {name} entry");
        error!("{msg}");
        Error::Jailer(msg)
    })
}
