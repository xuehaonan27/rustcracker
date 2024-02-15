use serde::{Deserialize, Serialize};

use crate::components::machine::MachineError;

use std::{os::{fd::FromRawFd, unix::fs::OpenOptionsExt}, path::PathBuf};

use nix::unistd::{access, AccessFlags, Gid, Uid};

use log::error;
pub trait Json<'a> {
    type Item;
    // fn from_json(s: &'a String) -> serde_json::Result<Self::Item>
    // where
    //     <Self as Json<'a>>::Item: Deserialize<'a>,
    // {
    //     let b: Self::Item = serde_json::from_str(s.as_str())?;
    //     Ok(b)
    // }

    fn from_json(s: &'a str) -> serde_json::Result<Self::Item>
    where
        <Self as Json<'a>>::Item: Deserialize<'a>,
    {
        let b: Self::Item = serde_json::from_str(s)?;
        Ok(b)
    }

    fn to_json(&self) -> serde_json::Result<String>
    where
        Self: Serialize,
    {
        let s: String = serde_json::to_string(self)?;
        Ok(s)
    }

    fn into_json(self) -> serde_json::Result<String>
    where
        Self: Serialize + Sized,
    {
        let s: String = serde_json::to_string(&self)?;
        Ok(s)
    }
}

pub const FIRECRACKER_BINARY_PATH: &'static str = "firecracker";
pub const FIRECRACKER_BINARY_OVERRIDE_ENV: &'static str = "FC_BIN";

pub const DEFAULT_JAILER_BINARY: &'static str = "jailer";
pub const JAILER_BINARY_OVERRIDE_ENV: &'static str = "FC_JAILER_BIN";

pub const DEFUALT_TUNTAP_NAME: &'static str = "fc-test-tap0";
pub const TUNTAP_OVERRIDE_ENV: &'static str = "FC_TAP";

pub const DATA_PATH_ENV: &'static str = "FC_DATA_PATH";
pub const SUDO_UID_ENV: &'static str = "SUDO_UID";
pub const SUDO_GID_ENV: &'static str = "SUDO_GID";

pub const DEFAULT_JAILER_PATH: &'static str = "/srv/jailer";
pub const ROOTFS_FOLDER_NAME: &'static str = "root";

pub const DEFAULT_SOCKET_PATH: &'static str = "/run/firecracker.socket";

pub const DEFAULT_USER_AGENT: &'static str = "rustcracker";
// as specified in http://man7.org/linux/man-pages/man8/ip-netns.8.html
pub const DEFAULT_NETNS_DIR: &'static str = "/var/run/netns";

// env name to make firecracker init timeout configurable
pub(super) const FIRECRACKER_INIT_TIMEOUT_ENV: &'static str = "RUSTCRACKER_INIT_TIMEOUT_SECONDS";
pub const DEFAULT_FIRECRACKER_INIT_TIMEOUT_SECONDS: f64 = 3.0;

// env name to make firecracker request timeout configurable
pub const FIRECRACKER_REQUEST_TIMEOUT_ENV: &'static str = "RUSTCRACKER_AGENT_TIMEOUT_SECONDS";
pub const DEFAULT_FIRECRACKER_REQUEST_TIMEOUT_SECONDS: f64 = 20.0;

// env name to overwrite async channel bound nums
pub const ASYNC_CHANNEL_BOUND_ENV: &'static str = "RUSTCRACKER_ASYNC_CHANNEL_BOUND_NUMS";
pub const DEFAULT_ASYNC_CHANNEL_BOUND_NUMS: usize = 16;

pub struct TestArgs {}

impl TestArgs {
    pub fn get_firecracker_binary_path() -> PathBuf {
        if let Ok(val) = std::env::var(FIRECRACKER_BINARY_OVERRIDE_ENV) {
            PathBuf::from(val)
        } else {
            Self::test_data_path().join(FIRECRACKER_BINARY_PATH)
        }
    }

    pub fn get_jailer_binary_path() -> PathBuf {
        if let Ok(val) = std::env::var(JAILER_BINARY_OVERRIDE_ENV) {
            PathBuf::from(val)
        } else {
            Self::test_data_path().join(DEFAULT_JAILER_BINARY)
        }
    }

    pub fn get_vmlinux_path() -> Result<PathBuf, MachineError> {
        let vmlinux_path = Self::test_data_path().join("./vmlinux");
        std::fs::metadata(&vmlinux_path).map_err(|e| {MachineError::FileMissing(format!(
            "Cannot find vmlinux file: {}\nVerify that you have a vmlinux file at {} or set the {} environment variable to the correct location",
            e.to_string(), vmlinux_path.display(), DATA_PATH_ENV,
        ))})?;
        Ok(vmlinux_path)
    }
    pub fn skip_tuntap() -> bool {
        false
    }
    pub fn test_data_path() -> PathBuf {
        "./testdata".into()
    }
    pub fn test_data_log_path() -> PathBuf {
        "logs".into()
    }
    pub fn test_data_bin() -> PathBuf {
        "bin".into()
    }
    pub fn test_root_fs() -> PathBuf {
        "root-drive.img".into()
    }
    pub fn test_balloon_memory() -> i64 {
        10
    }
    pub fn test_balloon_new_memory() -> i64 {
        6
    }
    pub fn test_balloon_deflate_on_oon() -> bool {
        true
    }
    pub fn test_stats_polling_interval_s() -> i64 {
        1
    }
    pub fn test_new_stats_polling_intervals() -> i64 {
        6
    }
}

pub fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

pub fn check_kvm() -> Result<(), MachineError> {
    access("/dev/kvm", AccessFlags::W_OK).map_err(|e| {
        error!("/dev/kvm is not writable {}", e);
        MachineError::FileAccess(format!("/dev/kvm is not writable {}", e))
    })?;
    Ok(())
}

pub fn copy_file(from: &PathBuf, to: &PathBuf, uid: u32, gid: u32) -> Result<(), MachineError> {
    std::fs::copy(from, to).map_err(|e| {
        MachineError::FileAccess(format!(
            "copy_file: Fail to copy file from {} to {}: {}",
            from.display(),
            to.display(),
            e.to_string()
        ))
    })?;
    nix::unistd::chown(to, Some(Uid::from_raw(uid)), Some(Gid::from_raw(gid))).map_err(|e| {
        MachineError::FileAccess(format!(
            "copy_file: Fail to chown file {}: {}",
            to.display(),
            e.to_string()
        ))
    })?;
    Ok(())
}

pub fn make_socket_path(test_name: &'static str) -> PathBuf {
    std::env::temp_dir().join(test_name.replace("/", "_")).join("fc.sock")
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum StdioTypes {
    // 空设备
    Null,
    // 进程管道的
    Piped,
    // 继承进程的
    Inherit,
    // 从指定文件打开的, std::fs::File
    From { path: PathBuf },
    // 从指定文件描述符打开的
    FromRawFd { fd: i32 },
}

impl StdioTypes {
    pub fn open_io(&self) -> Result<std::process::Stdio, MachineError> {
        match self {
            StdioTypes::Null => Ok(std::process::Stdio::null()),
            StdioTypes::Piped => Ok(std::process::Stdio::piped()),
            StdioTypes::Inherit => Ok(std::process::Stdio::inherit()),
            StdioTypes::From { path } => Ok(std::process::Stdio::from({
                let mut options = std::fs::OpenOptions::new();
                options.mode(0o644);
                options.open(&path).map_err(|e| {
                    error!(target: "StdioTypes: open_io", "fail to open {}: {}", path.display(), e);
                    MachineError::FileAccess(format!(
                        "fail to open {}: {}", path.display(), e
                    ))
                })?
            })),
            StdioTypes::FromRawFd { fd } => {
                Ok(unsafe { std::process::Stdio::from_raw_fd(fd.to_owned()) })
            }
        }
    }
}