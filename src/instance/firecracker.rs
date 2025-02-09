//! Option to launch firecracker

use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct FirecrackerOption {
    // Path to unix domain socket used by the API. [default: "/run/firecracker.socket"]
    api_sock: Option<PathBuf>,

    // Whether or not to load boot timer device for logging elapsed time since InstanceStart command.
    boot_timer: Option<bool>,

    // Path to a file that contains the microVM configuration in JSON format.
    config_file: Option<PathBuf>,

    // Print the data format version of the provided snapshot state file.
    describe_snapshot: Option<bool>,

    // Http API request payload max size, in bytes. [default: "51200"]
    http_api_max_payload_size: Option<usize>,

    // MicroVM unique identifier. [default: "anonymous-instance"]
    id: Option<String>,

    // Set the logger level.
    level: Option<String>,

    // Path to a fifo or a file used for configuring the logger on startup.
    log_path: Option<PathBuf>,

    // Path to a file that contains metadata in JSON format to add to the mmds.
    metadata: Option<PathBuf>,

    // Path to a fifo or a file used for configuring the metrics on startup.
    metrics_path: Option<PathBuf>,

    // Mmds data store limit, in bytes.
    mmds_size_limit: Option<PathBuf>,

    // Set the logger module filter.
    module: Option<String>,

    // Optional parameter which allows starting and using a microVM without an active API socket.
    no_api: Option<bool>,

    // Optional parameter which allows starting and using a microVM without seccomp filtering. Not recommended.
    no_seccomp: Option<bool>,

    // Parent process CPU time (wall clock, microseconds). This parameter is optional.
    parent_cpu_time_us: Option<usize>,

    // Optional parameter which allows specifying the path to a custom seccomp filter. For advanced users.
    seccomp_filter: Option<String>,

    // Whether or not to output the level in the logs.
    show_level: Option<bool>,

    // Whether or not to include the file path and line number of the log's origin.
    show_log_origin: Option<bool>,

    // Process start CPU time (wall clock, microseconds). This parameter is optional.
    start_time_cpu_us: Option<usize>,

    // Process start time (wall clock, microseconds). This parameter is optional.
    start_time_us: Option<usize>,
}

impl FirecrackerOption {
    pub fn new() -> Self {
        Default::default()
    }
}