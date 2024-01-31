use serde::{Deserialize, Serialize};

use crate::{client::machine::MachineError, model::balloon::Balloon};

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

pub trait Metadata {
    fn from_raw_string(value: String) -> Result<Self, &'static str>
    where
        Self: Sized;
    fn to_raw_string(&self) -> Result<String, &'static str>;
}

impl Metadata for Balloon {
    fn from_raw_string(value: String) -> Result<Self, &'static str> {
        Balloon::from_json(value.as_str())
            .map_err(|_| "fail to deserialize json to Balloon")
    }

    fn to_raw_string(&self) -> Result<String, &'static str> {
        self.to_json().map_err(|_| "fail to serialize to a json style string")
    }
}

impl Metadata for String {
    fn from_raw_string(value: String) -> Result<Self, &'static str>
        where
            Self: Sized {
        Ok(value)
    }
    fn to_raw_string(&self) -> Result<String, &'static str> {
        Ok(self.to_string())
    }
}

use std::path::PathBuf;

use nix::unistd::{access, AccessFlags, Gid, Uid};

use log::error;

pub const FIRECRACKER_BINARY_PATH: &'static str = "firecracker";
pub const FIRECRACKER_BINARY_OVERRIDE_ENV: &'static str = "FC_BIN";

pub const DEFAULT_JAILER_BINARY: &'static str = "jailer";
pub const JAILER_BINARY_OVERRIDE_ENV: &'static str = "FC_JAILER_BIN";

pub const DEFUALT_TUNTAP_NAME: &'static str = "fc-test-tap0";
pub const TUNTAP_OVERRIDE_ENV: &'static str = "FC_TAP";

pub const DATA_PATH_ENV: &'static str = "FC_DATA_PATH";
pub const SUDO_UID_ENV: &'static str = "SUDO_UID";
pub const SUDO_GID_ENV: &'static str = "SUDO_GID";


pub const DEFAULT_USER_AGENT: &'static str = "rustfire";
// as specified in http://man7.org/linux/man-pages/man8/ip-netns.8.html
pub const DEFAULT_NETNS_DIR: &'static str = "/var/run/netns";

// env name to make firecracker init timeout configurable
pub const FIRECRACKER_INIT_TIMEOUT_ENV: &'static str = "RUSTFIRE_INIT_TIMEOUT_SECONDS";
pub const DEFAULT_FIRECRACKER_INIT_TIMEOUT_SECONDS: usize = 3;

// env name to make firecracker request timeout configurable
pub const FIRECRACKER_REQUEST_TIMEOUT_ENV: &'static str = "RUSTFIRE_AGENT_TIMEOUT_SECONDS";
pub const DEFAULT_FIRECRACKER_REQUEST_TIMEOUT_SECONDS: usize = 3;

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