use std::{os::unix::fs::MetadataExt, path::PathBuf};

use nix::{
    fcntl::OFlag,
    sys::stat::Mode,
    unistd::{Gid, Uid},
};
use rustfire::{
    client::{
        handler::HandlersAdapter,
        jailer::{JailerConfig, StdioTypes},
        machine::{Config, Machine, MachineError},
    },
    model::{
        cpu_template::{self, CPUTemplate, CPUTemplateString},
        drive::Drive,
        logger::LogLevel,
        machine_configuration::MachineConfiguration,
    },
};
use tokio::sync::oneshot;

const FIRECRACKER_BINARY_PATH: &'static str = "firecracker";
const FIRECRACKER_BINARY_OVERRIDE_ENV: &'static str = "FC_TEST_BIN";
const DEFAULT_JAILER_BINARY: &'static str = "jailer";
const JAILER_BINARY_OVERRIDE_ENV: &'static str = "FC_TEST_JAILER_BIN";
const DEFUALT_TUNTAP_NAME: &'static str = "fc-test-tap0";
const TUNTAP_OVERRIDE_ENV: &'static str = "FC_TEST_TAP";
const TEST_DATA_PATH_ENV: &'static str = "FC_TEST_DATA_PATH";
const SUDO_UID: &'static str = "SUDO_UID";
const SUDO_GID: &'static str = "SUDO_GID";

struct TestArgs {
    pub(crate) skip_tuntap: bool,
    pub(crate) test_data_path: PathBuf,
    pub(crate) test_data_log_path: PathBuf,
    pub(crate) test_data_bin: PathBuf,

    pub(crate) test_root_fs: PathBuf,

    pub(crate) test_balloon_memory: i64,
    pub(crate) test_balloon_new_memory: i64,
    pub(crate) test_balloon_deflate_on_oon: bool,
    pub(crate) test_stats_polling_interval_s: i64,
    pub(crate) test_new_stats_polling_intervals: i64,
}
impl Default for TestArgs {
    fn default() -> Self {
        Self {
            skip_tuntap: false,
            test_data_path: "./testdata".into(),
            test_data_log_path: "logs".into(),
            test_data_bin: "bin".into(),
            test_root_fs: "root-drive.img".into(),
            test_balloon_memory: 10,
            test_balloon_new_memory: 6,
            test_balloon_deflate_on_oon: true,
            test_stats_polling_interval_s: 1,
            test_new_stats_polling_intervals: 6,
        }
    }
}

fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

fn check_kvm() -> Result<(), MachineError> {
    todo!()
}

fn copy_file(from: &PathBuf, to: &PathBuf, uid: u32, gid: u32) -> Result<(), MachineError> {
    std::fs::copy(from, to).map_err(|e| {
        MachineError::FileError(format!(
            "copy_file: Fail to copy file from {} to {}: {}",
            from.display(),
            to.display(),
            e.to_string()
        ))
    })?;
    nix::unistd::chown(to, Some(Uid::from_raw(uid)), Some(Gid::from_raw(gid))).map_err(|e| {
        MachineError::FileError(format!(
            "copy_file: Fail to chown file {}: {}",
            to.display(),
            e.to_string()
        ))
    })?;
    Ok(())
}

#[test]
fn test_new_machine() {
    init();

    let config: Config = Config::default()
        .with_machine_config(
            MachineConfiguration::default()
                .with_vcpu_count(1)
                .with_mem_size_mib(100)
                .with_cpu_template(&CPUTemplate(cpu_template::CPUTemplateString::T2))
                .set_hyperthreading(false),
        )
        .set_disable_validation(true);
    let (_send, exit_recv) = async_channel::bounded(64);
    let (_send, sig_recv) = async_channel::bounded(64);
    let m = Machine::new(config, exit_recv, sig_recv, 10, 100);
    let _m = match m {
        Ok(m) => m,
        Err(e) => panic!("failed to create new machine: {}", e),
    };
}

#[test]
fn test_jailer_micro_vm_execution() -> Result<(), MachineError> {
    init();

    let test_args = TestArgs::default();
    let log_path = test_args
        .test_data_log_path
        .join("test_jailer_micro_vm_execution");
    std::fs::create_dir_all(&log_path).map_err(|e| {
        MachineError::FileError(format!(
            "fail to create log path {}: {}",
            log_path.display(),
            e.to_string()
        ))
    })?;
    // assert 0o777

    let jailer_uid = 123;
    let jailer_gid = 100;

    // use temp directory
    let tmpdir = std::env::temp_dir().join("jailer-test");

    let vmlinux_path = tmpdir.join("vmlinux");
    copy_file(
        &test_args.test_data_path.join("vmlinux"),
        &vmlinux_path,
        jailer_uid,
        jailer_gid,
    )?;

    let root_drive_path = tmpdir.join("root-drive.img");
    copy_file(
        &test_args.test_root_fs,
        &root_drive_path,
        jailer_uid,
        jailer_gid,
    )?;

    let ncpus = 2;
    let cpu_template = CPUTemplate(CPUTemplateString::T2);
    let memsz = 256;

    // shot names and directory to prevent SUN_LEN error
    let id = "b";
    let jail_test_path = tmpdir.to_owned();
    std::fs::create_dir_all(&jail_test_path).map_err(|e| {
        MachineError::FileError(format!(
            "fail to create log path {}: {}",
            jail_test_path.display(),
            e.to_string()
        ))
    })?;
    // assert 0o777

    let socket_path = "test_jailer_micro_vm_execution.socket";
    let log_fifo = tmpdir.join("firecracker.log");
    let metrics_fifo = tmpdir.join("firecracker-metrics");
    let captured_log = tmpdir.join("writer.fifo");

    // let fw = std::fs::OpenOptions::new()
    //     .create(true)
    //     .read(true)
    //     .write(true)
    //     .mode(0o600)
    //     .open(&captured_log)
    //     .map_err(|e| {
    //         MachineError::FileError(format!(
    //             "fail to open the path {}: {}",
    //             captured_log.display(),
    //             e.to_string(),
    //         ))
    //     })?;

    let fw = nix::fcntl::open(
        &captured_log,
        OFlag::O_CREAT | OFlag::O_RDWR,
        Mode::from_bits(0o600).ok_or(MachineError::FileError(format!(
            "fail to convert '600' to Mode: {}",
            captured_log.display(),
        )))?,
    )
    .map_err(|e| {
        MachineError::FileError(format!(
            "fail to open th path {}: {}",
            captured_log.display(),
            e.to_string()
        ))
    })?;

    let log_fd = nix::fcntl::open(
        &log_path.join("test_jailer_micro_vm_execution.log"),
        OFlag::O_CREAT | OFlag::O_RDWR,
        Mode::from_bits(0o666).ok_or(MachineError::FileError(
            "fail to convert '0o666' to Mode".to_string(),
        ))?,
    )
    .map_err(|e| {
        MachineError::FileError(format!("failed to create log file: {}", e.to_string()))
    })?;

    let cfg = Config {
        socket_path: Some(socket_path.into()),
        log_path: None,
        log_fifo: Some(log_fifo.to_owned()),
        log_level: Some(LogLevel::Debug),
        kernel_image_path: Some(vmlinux_path.to_owned()),
        machine_cfg: Some(MachineConfiguration {
            vcpu_count: ncpus,
            cpu_template: Some(cpu_template),
            mem_size_mib: memsz,
            ht_enabled: Some(false),
            track_dirty_pages: None,
        }),
        drives: Some(vec![Drive {
            drive_id: 1.to_string(),
            is_root_device: true,
            is_read_only: false,
            path_on_host: root_drive_path,
            partuuid: None,
            cache_type: None,
            rate_limiter: None,
            io_engine: None,
            socket: None,
        }]),
        jailer_cfg: Some(JailerConfig {
            jailer_binary: Some(test_args.test_data_path.join(DEFAULT_JAILER_BINARY)),
            gid: Some(jailer_gid),
            uid: Some(jailer_uid),
            numa_node: Some(0),
            id: Some(id.to_string()),
            chroot_base_dir: Some(jail_test_path.to_owned()),
            exec_file: Some(test_args.test_data_path.join(FIRECRACKER_BINARY_PATH)),
            chroot_strategy: Some(HandlersAdapter::NaiveChrootStrategy {
                rootfs: "".into(),
                kernel_image_path: vmlinux_path.to_owned(),
            }),
            stdout: Some(StdioTypes::FromRawFd { fd: log_fd }),
            stderr: Some(StdioTypes::FromRawFd { fd: log_fd }),
            daemonize: None,
            stdin: None,
        }),
        fifo_log_writer: Some(StdioTypes::FromRawFd { fd: fw }),
        metrics_path: None,
        metrics_fifo: None,
        initrd_path: None,
        kernel_args: None,
        network_interfaces: None,
        vsock_devices: None,
        disable_validation: true,
        vmid: None,
        net_ns: None,
        forward_signals: None,
        seccomp_level: None,
        mmds_address: None,
    };

    std::fs::metadata(&vmlinux_path).map_err(|e| {
        MachineError::FileError(format!(
            "Cannot find vmlinux file: {}\nVerify that you have a vmlinux file at {}",
            e.to_string(),
            vmlinux_path.display()
        ))
    })?;

    let kernel_image_info =
        std::fs::metadata(cfg.kernel_image_path.as_ref().unwrap()).map_err(|e| {
            MachineError::FileError(format!("failed to stat kernel image: {}", e.to_string()))
        })?;

    if kernel_image_info.uid() != jailer_uid || kernel_image_info.gid() != jailer_gid {
        return Err(MachineError::FileError(format!(
            "Kernel image does not have the proper UID or GID\nTo fix this simply run:\nsudo chown {}:{} {}",
            jailer_uid, jailer_gid, cfg.kernel_image_path.as_ref().unwrap().display()
        )));
    }

    for drive in cfg.drives.as_ref().unwrap() {
        let drive_image_info = std::fs::metadata(&drive.path_on_host).map_err(|e| MachineError::FileError(format!(
            "failed to stat drive: {}", e.to_string()
        )))?;

        if drive_image_info.uid() != jailer_uid || drive_image_info.gid() != jailer_gid {
            return Err(MachineError::FileError(format!(
                "Drive does not have the proper uid or gid\nTo fix this simply run:\nsudo chown {}:{} {}",
                jailer_uid, jailer_gid, drive.path_on_host.display()    
            )))
        }
    }

    let (_send, exit_recv) = async_channel::bounded(64);
    let (_send, sig_recv) = async_channel::bounded(64);
    let mut m = Machine::new(cfg, exit_recv, sig_recv, 10, 60).map_err(|e| MachineError::Initialize(format!(
        "Failed to start VMM: {}", e.to_string()
    )))?;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async move {
        m.start().await.map_err(|e| MachineError::Execute(format!("failed to start VMM: {}", e.to_string()))).expect("fail to start VMM");
        m.stop_vmm().await.expect("cannot stop vmm");
    });

    // Closing:
    nix::unistd::close(fw).map_err(|e| {
        MachineError::FileError(format!(
            "double closing {}: {}",
            captured_log.display(),
            e.to_string()
        ))
    })?;
    std::fs::remove_file(&captured_log).map_err(|e| {
        MachineError::FileError(format!(
            "fail to remove file {}: {}",
            captured_log.display(),
            e.to_string()
        ))
    })?;
    std::fs::remove_file(jail_test_path.join("firecracker").join(socket_path)).map_err(|e| {
        MachineError::FileError(format!("fail to remove socket file: {}", e.to_string()))
    })?;
    std::fs::remove_file(&log_fifo).map_err(|e| {
        MachineError::FileError(format!(
            "fail to remove log fifo file at {}: {}",
            log_fifo.display(),
            e.to_string()
        ))
    })?;
    std::fs::remove_file(&metrics_fifo).map_err(|e| {
        MachineError::FileError(format!(
            "fail to remove metrics fifo file at {}: {}",
            metrics_fifo.display(),
            e.to_string()
        ))
    })?;
    std::fs::remove_dir_all(&tmpdir).map_err(|e| {
        MachineError::FileError(format!(
            "fail to remove dir at {}: {}",
            tmpdir.display(),
            e.to_string()
        ))
    })?;
    nix::unistd::close(log_fd).map_err(|e| {
        MachineError::FileError(format!("double closing log file: {}", e.to_string()))
    })?;

    let info = std::fs::metadata(&captured_log);
    assert!(info.is_err());

    Ok(())
}

#[test]
fn test_micro_vm_execution() -> Result<(), MachineError> {
    init();
    Ok(())
}

#[test]
fn test_start_vmm() {
    init();
    
}