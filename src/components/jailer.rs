use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::utils::StdioTypes;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct JailerConfig {
    // GID the jailer switches to as it execs the target binary.
    pub gid: Option<u32>,

    // UID the jailer switches to as it execs the target binary.
    pub uid: Option<u32>,

    // ID is the unique VM identification string, which may contain alphanumeric
    // characters and hyphens. The maximum id length is currently 64 characters
    pub id: Option<String>,

    // NumaNode represents the NUMA node the process gets assigned to.
    pub numa_node: Option<usize>,

    // ExecFile is the path to the Firecracker binary that will be exec-ed by
    // the jailer. The user can provide a path to any binary, but the interaction
    // with the jailer is mostly Firecracker specific.
    pub exec_file: Option<PathBuf>,

    // JailerBinary specifies the jailer binary to be used for setting up the
    // Firecracker VM jail.
    // If not specified it defaults to "jailer".
    pub jailer_binary: Option<PathBuf>,

    // ChrootBaseDir represents the base folder where chroot jails are built. The
    // default is /srv/jailer
    pub chroot_base_dir: Option<PathBuf>,

    //  Daemonize is set to true, call setsid() and redirect STDIN, STDOUT, and
    //  STDERR to /dev/null
    pub daemonize: Option<bool>,

    // ChrootStrategy will dictate how files are transfered to the root drive.
    // pub chroot_strategy: Option<HandlersAdapter>,

    // Stdout specifies the IO writer for STDOUT to use when spawning the jailer.
    // pub(crate) stdout: Option<std::process::Stdio>,
    pub stdout: Option<StdioTypes>,

    // Stderr specifies the IO writer for STDERR to use when spawning the jailer.
    pub stderr: Option<StdioTypes>,

    // Stdin specifies the IO reader for STDIN to use when spawning the jailer.
    pub stdin: Option<StdioTypes>,
}
