use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::utils::Json;

/// Describes the configuration option for the metrics capability.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metrics {
    /// Path to the named pipe or file where the JSON-formatted metrics are flushed.
    /// Required: true
    #[serde(rename = "metrics_path")]
    pub metrics_path: PathBuf,
}

impl<'a> Json<'a> for Metrics {
    type Item = Metrics;
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            metrics_path: "".into(),
        }
    }
}

impl Metrics {
    pub fn with_metrics_path(mut self, metrics_path: &PathBuf) -> Self {
        self.metrics_path = metrics_path.clone();
        self
    }
}
