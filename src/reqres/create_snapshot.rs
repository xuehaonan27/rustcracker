use crate::models::snapshot_create_params::SnapshotCreateParams;

use super::FirecrackerRequest;

pub struct CreateSnapshot {
    payload: SnapshotCreateParams,
}

impl CreateSnapshot {
    pub fn new(payload: SnapshotCreateParams) -> Self {
        Self { payload }
    }
}
