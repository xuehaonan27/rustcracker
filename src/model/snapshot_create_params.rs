use serde::{Serialize, Deserialize};

use crate::utils::Json;
#[derive(Serialize, Deserialize)]
pub struct SnapshotCreateParams {
    // Path to the file that will contain the guest memory.
	// Required: true
    mem_file_path: String,

    // Path to the file that will contain the microVM state.
	// Required: true
    snapshot_path: String,

    // Type of snapshot to create. It is optional and by default, a full snapshot is created.
	// Enum: [Full Diff]
    snapshot_type: Option<String>,

    // The microVM version for which we want to create the snapshot.
    // It is optional and it defaults to the current version.
    version: Option<String>,
}


impl<'a> Json<'a> for SnapshotCreateParams {
    type Item = SnapshotCreateParams;
}