/*
@File: model/balloon_stats.rs
@Author: Mugen_Cyaegha (Xue Haonan)
*/
use crate::utils::Json;
use serde::{Deserialize, Serialize};

/// Describes the balloon device statistics.
/// 
/// This structure represents the return value requested
/// by `GET /balloon/statistics`, which describes detailed
/// information of the balloon device.
#[derive(Serialize, Deserialize, Clone)]
pub struct BalloonStatistics {
    // Target number of pages the device aims to hold.
    // Required: true
    target_pages: u64,

    // Actual number of pages the device is holding.
    // Required: true
    actual_pages: u64,

    // Target amount of memory (in MiB) the device aims to hold.
    // Required: true
    target_mib: u64,

    // Actual amount of memory (in MiB) the device is holding.
    // Required: true
    actual_mib: u64,

    // The amount of memory that has been swapped in (in bytes).
    swap_in: u64,

    // The amount of memory that has been swapped out to disk (in bytes).
    swap_out: u64,

    // The number of major page faults that have occurred.
    major_faults: u64,

    // The number of minor page faults that have occurred.
    minor_faults: u64,

    // The amount of memory not being used for any purpose (in bytes).
    free_memory: u64,

    // The total amount of memory available (in bytes).
    total_memory: u64,

    // An estimate of how much memory is available (in bytes) for starting new applications
    // without pushing the system to swap;
    available_memory: u64,

    // The amount of memory, in bytes, that can be quickly reclaimed without additional I/O.
    // Typically these pages are used for caching files from disk.
    disk_caches: u64,

    // The number of successful hugetlb page allocations in the guest.
    hugetlb_allocations: u64,

    // The number of failed hugetlb page allocations in the guest.
    hugetlb_failures: u64,
}

impl<'a> Json<'a> for BalloonStatistics {
    type Item = BalloonStatistics;
}
