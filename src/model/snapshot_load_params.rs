use serde::{Serialize, Deserialize};

use crate::utils::Json;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SnapshotLoadParams {
    // Enable support for incremental (diff) snapshots
    // by tracking dirty guest pages.
    enable_diff_snapshots: Option<bool>,

    // Path to the file that contains the guest memory to be loaded.
	// Required: true
    mem_file_path: String,

    // When set to true, the vm is also resumed
    // if the snapshot load is successful.
    resume_vm: Option<bool>,

    // Path to the file that contains the microVM state to be loaded.
	// Required: true
    snapshot_path: String,
}


impl<'a> Json<'a> for SnapshotLoadParams {
    type Item = SnapshotLoadParams;
}