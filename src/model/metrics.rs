use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::utils::Json;
#[derive(Serialize, Deserialize)]
pub struct Metrics {
    // Path to the named pipe or file where the JSON-formatted metrics are flushed.
    // Required: true
    metrics_path: PathBuf,
}

impl<'a> Json<'a> for Metrics {
    type Item = Metrics;
}
