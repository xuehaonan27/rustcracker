//! Option to launch jailer

use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct JailerOption {
    // File path to exec into.
    exec_file: Option<PathBuf>,

    // The group identifier the jailer switches to after exec.
    gid: Option<usize>,

    // Jail ID.
    id: Option<usize>,

    // The user identifier the jailer switches to after exec.
    uid: Option<usize>,

    // Cgroup and value to be set by the jailer. It must follow this format: <cgroup_file>=<value> (e.g cpu.shares=10). This argument can be used multiple times to add multiple cgroups.
    cgroup: Option<String>,

    // Select the cgroup version used by the jailer. [default: "1"]
    cgroup_version: Option<usize>,

    // The base folder where chroot jails are located. [default: "/srv/jailer"]
    chroot_base_dir: Option<PathBuf>,

    // Daemonize the jailer before exec, by invoking setsid(), and redirecting the standard I/O file descriptors to /dev/null.
    daemonize: Option<bool>,

    // Path to the network namespace this microVM should join.
    netns: Option<PathBuf>,

    // Exec into a new PID namespace.
    new_pid_ns: Option<bool>,

    // Parent cgroup in which the cgroup of this microvm will be placed.
    parent_cgroup: Option<String>,

    
}
