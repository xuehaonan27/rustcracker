use std::os::unix::fs::MetadataExt;

use nix::{
    fcntl::OFlag,
    sys::stat::Mode,
};
use rustcracker::{
    components::{
        command_builder::VMMCommandBuilder, jailer::JailerConfig, machine::{Config, Machine, MachineError, MachineMessage}
    }, model::{
        cpu_template::{self, CPUTemplate, CPUTemplateString}, drive::Drive, logger::LogLevel, machine_configuration::MachineConfiguration, network_interface::NetworkInterface
    }, utils::{check_kvm, copy_file, init, StdioTypes, TestArgs, DEFAULT_JAILER_BINARY, FIRECRACKER_BINARY_PATH}
};
use log::{error, info};

#[test]
fn test_new_machine() -> Result<(), MachineError> { 
    init();
    check_kvm()?;
    let config = Config {
        socket_path: Some("foo/bar".into()),
        machine_cfg:  Some(MachineConfiguration::default()
                    .with_vcpu_count(1)
                    .with_mem_size_mib(100)
                    .with_cpu_template(&CPUTemplate(cpu_template::CPUTemplateString::T2))
                    .set_hyperthreading(false)),
        disable_validation: true,
        ..Default::default()
    };
    // let config: Config = Config::default()
    //     .with_machine_config(
    //         MachineConfiguration::default()
    //             .with_vcpu_count(1)
    //             .with_mem_size_mib(100)
    //             .with_cpu_template(&CPUTemplate(cpu_template::CPUTemplateString::T2))
    //             .set_hyperthreading(false),
    //     )
    //     .set_disable_validation(true);
    let (_send, exit_recv) = async_channel::bounded(64);
    // let (_send, sig_recv) = async_channel::bounded(64);
    Machine::new(config, exit_recv, 10, 100)?;
    Ok(())
}

#[tokio::test]
async fn test_jailer_micro_vm_execution() -> Result<(), MachineError> {
    init();
    check_kvm()?;
    let log_path = TestArgs::
        test_data_log_path()
        .join("test_jailer_micro_vm_execution");
    std::fs::create_dir_all(&log_path).map_err(|e| {
        MachineError::FileCreation(format!(
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
    std::fs::create_dir_all(&tmpdir).map_err(|e| {
        MachineError::FileCreation(format!(
            "fail to create dir path {}: {}",
            tmpdir.display(),
            e.to_string()
        ))
    })?;

    let vmlinux_path = tmpdir.join("vmlinux");
    copy_file(
        &TestArgs::test_data_path().join("vmlinux"),
        &vmlinux_path,
        jailer_uid,
        jailer_gid,
    )?;

    let root_drive_path = tmpdir.join("root-drive.img");
    copy_file(
        &TestArgs::test_data_path().join(TestArgs::test_root_fs()),
        &root_drive_path,
        jailer_uid,
        jailer_gid,
    )?;

    let ncpus = 2;
    let cpu_template = CPUTemplate(CPUTemplateString::None);
    let memsz = 256;

    // shot names and directory to prevent SUN_LEN error
    let id = "b";
    let jail_test_path = tmpdir.to_owned();
    std::fs::create_dir_all(&jail_test_path).map_err(|e| {
        MachineError::FileCreation(format!(
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

    // let fw = nix::fcntl::open(
    //     &captured_log,
    //     OFlag::O_CREAT | OFlag::O_RDWR,
    //     Mode::from_bits(0o600).ok_or(MachineError::FileAccess(format!(
    //         "fail to convert '600' to Mode: {}",
    //         captured_log.display(),
    //     )))?,
    // )
    // .map_err(|e| {
    //     MachineError::FileAccess(format!(
    //         "fail to open th path {}: {}",
    //         captured_log.display(),
    //         e.to_string()
    //     ))
    // })?;

    let log_fd = nix::fcntl::open(
        &log_path.join("test_jailer_micro_vm_execution.log"),
        OFlag::O_CREAT | OFlag::O_RDWR,
        Mode::from_bits(0o666).ok_or(MachineError::FileAccess(
            "fail to convert '0o666' to Mode".to_string(),
        ))?,
    )
    .map_err(|e| {
        MachineError::FileCreation(format!("failed to create log file: {}", e.to_string()))
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
            jailer_binary: Some(TestArgs::test_data_path().join(DEFAULT_JAILER_BINARY)),
            gid: Some(jailer_gid),
            uid: Some(jailer_uid),
            numa_node: Some(0),
            id: Some(id.to_string()),
            chroot_base_dir: Some(jail_test_path.to_owned()),
            exec_file: Some(TestArgs::test_data_path().join(FIRECRACKER_BINARY_PATH)),
            // chroot_strategy: Some(HandlersAdapter::NaiveChrootStrategy {
            //     rootfs: "".into(),
            //     kernel_image_path: vmlinux_path.to_owned(),
            // }),
            stdout: Some(StdioTypes::FromRawFd { fd: log_fd }),
            stderr: Some(StdioTypes::FromRawFd { fd: log_fd }),
            daemonize: Some(false),
            stdin: None,
        }),
        fifo_log_writer: Some(captured_log.to_owned()),
        metrics_path: None,
        metrics_fifo: Some(metrics_fifo.to_owned()),
        initrd_path: None,
        kernel_args: Some("".to_string()),
        network_interfaces: None,
        vsock_devices: Some(vec![]),
        disable_validation: true,
        vmid: None,
        net_ns: None,
        forward_signals: None,
        seccomp_level: Some(0),
        mmds_address: None,
        balloon: None,
        init_metadata: None,
        ..Default::default()
    };

    std::fs::metadata(&vmlinux_path).map_err(|e| {
        MachineError::FileMissing(format!(
            "Cannot find vmlinux file: {}\nVerify that you have a vmlinux file at {}",
            e.to_string(),
            vmlinux_path.display()
        ))
    })?;

    let kernel_image_info =
        std::fs::metadata(cfg.kernel_image_path.as_ref().unwrap()).map_err(|e| {
            MachineError::FileMissing(format!("failed to stat kernel image: {}", e.to_string()))
        })?;

    if kernel_image_info.uid() != jailer_uid || kernel_image_info.gid() != jailer_gid {
        return Err(MachineError::FileAccess(format!(
            "Kernel image does not have the proper UID or GID\nTo fix this simply run:\nsudo chown {}:{} {}",
            jailer_uid, jailer_gid, cfg.kernel_image_path.as_ref().unwrap().display()
        )));
    }

    for drive in cfg.drives.as_ref().unwrap() {
        let drive_image_info = std::fs::metadata(&drive.path_on_host).map_err(|e| MachineError::FileAccess(format!(
            "failed to stat drive: {}", e.to_string()
        )))?;

        if drive_image_info.uid() != jailer_uid || drive_image_info.gid() != jailer_gid {
            return Err(MachineError::FileAccess(format!(
                "Drive does not have the proper uid or gid\nTo fix this simply run:\nsudo chown {}:{} {}",
                jailer_uid, jailer_gid, drive.path_on_host.display()    
            )))
        }
    }

    let (_send, exit_recv) = async_channel::bounded(64);
    // let (_send, sig_recv) = async_channel::bounded(64);
    let mut m = Machine::new(cfg, exit_recv, 10, 10).map_err(|e| MachineError::Initialize(format!(
        "Failed to start VMM: {}", e.to_string()
    )))?;

    // let rt = tokio::runtime::Runtime::new().unwrap();
    // rt.block_on(async move {
    m.start().await.map_err(|e| MachineError::Execute(format!("failed to start VMM: {}", e.to_string()))).expect("fail to start VMM");
    m.stop_vmm().await.expect("cannot stop vmm");
    // });

    // Closing:
    // nix::unistd::close(fw).map_err(|e| {
    //     MachineError::FileRemoving(format!(
    //         "double closing {}: {}",
    //         captured_log.display(),
    //         e.to_string()
    //     ))
    // })?;

    std::fs::remove_file(&captured_log).map_err(|e| {
        MachineError::FileRemoving(format!(
            "fail to remove file {}: {}",
            captured_log.display(),
            e.to_string()
        ))
    })?;

    
    std::fs::remove_file(jail_test_path.join("firecracker").join(id).join("root").join(socket_path)).map_err(|e| {
        MachineError::FileRemoving(format!("fail to remove socket file at {}: {}", jail_test_path.join("firecracker").join(id).join("root").join(socket_path).display(), e.to_string()))
    })?;
    std::fs::remove_file(&log_fifo).map_err(|e| {
        MachineError::FileRemoving(format!(
            "fail to remove log fifo file at {}: {}",
            log_fifo.display(),
            e.to_string()
        ))
    })?;
    std::fs::remove_file(&metrics_fifo).map_err(|e| {
        MachineError::FileRemoving(format!(
            "fail to remove metrics fifo file at {}: {}",
            metrics_fifo.display(),
            e.to_string()
        ))
    })?;
    std::fs::remove_dir_all(&tmpdir).map_err(|e| {
        MachineError::FileRemoving(format!(
            "fail to remove dir at {}: {}",
            tmpdir.display(),
            e.to_string()
        ))
    })?;
    nix::unistd::close(log_fd).map_err(|e| {
        MachineError::FileRemoving(format!("double closing log file: {}", e.to_string()))
    })?;

    let info = std::fs::metadata(&captured_log);
    assert!(info.is_err());

    Ok(())
}

#[tokio::test]
async fn test_micro_vm_execution() -> Result<(), MachineError> {
    init();
    check_kvm()?;
    let ncpus = 2;
    let cpu_template = CPUTemplateString::T2;
    let mem_sz = 256;

    let dir = std::env::temp_dir().join("test_micro_vm_execution");
    std::fs::create_dir_all(&dir).map_err(|e| {
        MachineError::FileCreation(format!("fail to create directory {}: {}", dir.display(), e.to_string()))
    })?;
    let socket_path = dir.join("test_micro_vm_execution.sock");
    let log_fifo = dir.join("firecracker.log");
    let metrics_fifo = dir.join("firecracker-metrics");
    let captured_log = dir.join("writer.fifo");

    // let fw = nix::fcntl::open(&captured_log, OFlag::O_CREAT | OFlag::O_RDWR, Mode::S_IRUSR | Mode::S_IWUSR).map_err(|e| {
    //     MachineError::FileAccess(format!(
    //         "fail to open file {}: {}", captured_log.display(), e.to_string()
    //     ))
    // })?;

    let vmlinux_path = dir.join("vmlinux");
    // let network_ifaces = UniNetworkInterfaces(vec![
    //     UniNetworkInterface {
    //         static_configuration: Some(StaticNetworkConfiguration{mac_address:"01-23-45-67-89-AB-CD-EF".to_string(),host_dev_name:Some("tap0".to_string()),ip_configuration: None}),
    //         cni_configuration: None,
    //         allow_mmds: None,
    //         in_rate_limiter: None,
    //         out_rate_limiter: None,
    //     }
    // ]);
    let network_iface = NetworkInterface {
        guest_mac: Some("01-23-45-67-89-AB-CD-EF".to_string()),
        host_dev_name: "tap0".into(),
        iface_id: "0".to_string(),
        rx_rate_limiter: None,
        tx_rate_limiter: None,
        // allow_mmds_requests: None,
    };

    let cfg = Config {
        socket_path: Some(socket_path.to_owned()),
        log_fifo: Some(log_fifo),
        log_path: None,
        log_level: Some(LogLevel::Debug),
        metrics_fifo: Some(metrics_fifo),
        metrics_path: None,
        machine_cfg: Some(MachineConfiguration {
        vcpu_count: ncpus,
        cpu_template: Some(CPUTemplate(cpu_template)),
        ht_enabled: Some(false),
        mem_size_mib: mem_sz,
        track_dirty_pages: None,
                }),
        disable_validation: true,
        network_interfaces: Some(vec![network_iface]),
        fifo_log_writer: Some(captured_log),

        kernel_args: None,
        kernel_image_path: Some(vmlinux_path),
        initrd_path: None,
        drives: None,
        vsock_devices: None,
        jailer_cfg: None,
        vmid: None,
        net_ns: None,
        forward_signals: None,
        seccomp_level: None,
        mmds_address: None,
        init_metadata: None,
        balloon: None,
        ..Default::default()
    };

    let cmd = VMMCommandBuilder::new().with_socket_path(&socket_path).with_bin(&TestArgs::get_firecracker_binary_path()).build();
    log::debug!("{:#?}", cmd);
    let (exit_send, exit_recv) = async_channel::bounded(64);
    // let (_send, sig_recv) = async_channel::bounded(64);
    let mut m = Machine::new(cfg, exit_recv, 10, 500)?;
    m.set_command(cmd.into());

    // m.clear_validation();
    
    let join_handle = tokio::spawn(async move {
        if m.start_vmm_test().await.is_err() {
            error!("fail to start vmm");
            panic!("fail to start vmm");
        }

        // test_attach_root_drive(&mut m).await;

        if m.wait().await.is_err() {
            error!("fail to wait vmm");
            panic!("fail to wait vmm");
        }
        if m.stop_vmm().await.is_err() {
            error!("fail to stop vmm");
        }
    });
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    exit_send.send(MachineMessage::StopVMM).await.unwrap();
    info!("exit message sent");
    tokio::join!(join_handle).0.unwrap();

    // nix::unistd::close(fw).map_err(|e| {
    //     MachineError::FileRemoving(format!(
    //         "double closing {}: {}",
    //         captured_log.display(),
    //         e.to_string()
    //     ))
    // })?;

    std::fs::remove_dir_all(&dir).map_err(|e| {
        MachineError::FileRemoving(format!(
            "fail to remove dir at {}: {}",
            dir.display(),
            e.to_string()
        ))
    })?;
    Ok(())
}

#[tokio::test]
async fn test_start_vmm() -> Result<(), MachineError> {
    init();
    check_kvm()?;
    // let socket_path = make_socket_path("test_start_vmm");
    let test_name = "test_start_vmm";
    let dir_path = std::env::temp_dir().join(test_name.replace("/", "_"));
    std::fs::create_dir_all(&dir_path).map_err(|e| {
        MachineError::FileCreation(format!("fail to create directory {}: {}", dir_path.display(), e.to_string()))
    })?;
    let socket_path = dir_path.join("fc.sock");

    let cfg = Config {
        socket_path: Some(socket_path.to_owned()),
        machine_cfg: Some(MachineConfiguration::default()),
        disable_validation: true,
        ..Default::default()
    };

    let cmd = VMMCommandBuilder::new().with_socket_path(&socket_path).with_bin(&TestArgs::get_firecracker_binary_path()).build();
    
    let (exit_send, exit_recv) = async_channel::bounded(64);
    // let (sig_send, sig_recv) = async_channel::bounded(64);
    let mut m = Machine::new(cfg, exit_recv, 10, 60)?;
    m.set_command(cmd.into());

    // m.clear_validation();

    let (send, recv) = tokio::sync::oneshot::channel();
    tokio::select! {
        output = m.start_vmm_test() => {
            send.send(output).unwrap();
        }
        _ = tokio::time::sleep(tokio::time::Duration::from_millis(250)) => {
            info!("firecracker ran for 250ms and still start_vmm doesn't return");
            m.stop_vmm().await.expect("fail to stop_vmm!");
        }
    };

    // stop_vmm force anyway
    _ = m.stop_vmm_force().await;

    let msg = recv.await;
    info!("start_vmm sent: {:#?}", msg);

    // close channels
    exit_send.close();
    // sig_send.close();

    // delete socket path
    std::fs::remove_file(&socket_path).map_err(|e| {
        MachineError::FileRemoving(format!("fail to remove socket {}: {}", socket_path.display(), e.to_string()))
    })?;

    Ok(())
}

#[tokio::test]
async fn test_start_once() -> Result<(), MachineError> {
    init();
    check_kvm()?;

    // let socket_path = make_socket_path("test_start_once");
    let test_name = "test_start_once";
    let dir_path = std::env::temp_dir().join(test_name.replace("/", "_"));
    std::fs::create_dir_all(&dir_path).map_err(|e| {
        MachineError::FileCreation(format!("fail to create directory {}: {}", dir_path.display(), e.to_string()))
    })?;
    let socket_path = dir_path.join("fc.sock");

    let cfg = Config {
        socket_path: Some(socket_path.to_owned()),
        disable_validation: true,
        kernel_image_path: Some(TestArgs::get_vmlinux_path()?),
        machine_cfg: Some(MachineConfiguration{
            vcpu_count: 1,
            mem_size_mib: 64,
            cpu_template: Some(CPUTemplate(CPUTemplateString::None)),
            ht_enabled: Some(false),
            track_dirty_pages: None,
        }),
        kernel_args: Some("".to_string()),
        net_ns: Some("".into()),
        network_interfaces: None,
        ..Default::default()
    };

    let cmd = VMMCommandBuilder::new().with_socket_path(&socket_path).with_bin(&TestArgs::get_firecracker_binary_path()).build();
    let (exit_send, exit_recv) = async_channel::bounded(64);
    // let (sig_send, sig_recv) = async_channel::bounded(64);
    let mut m = Machine::new(cfg, exit_recv, 10, 60)?;
    m.set_command(cmd.into());

    let (send1, recv1) = tokio::sync::oneshot::channel();
    let (send2, recv2) = tokio::sync::oneshot::channel();

    tokio::select! {
        output = m.start() => {
            let res = m.start().await;
            assert!(res.is_err());
            send1.send(output).unwrap();
            send2.send(res).unwrap();
        }
        _ = tokio::time::sleep(tokio::time::Duration::from_millis(250)) => {
            info!("firecracker ran for 250ms and still start doesn't return");
            m.stop_vmm().await.expect("fail to stop_vmm!");
        }
    }

    _ = m.stop_vmm_force().await;

    let msg1 = recv1.await;
    info!("start1 sent: {:#?}", msg1);
    let msg2 = recv2.await;
    info!("start2 sent: {:#?}", msg2);

    // close channels
    exit_send.close();
    // sig_send.close();

    // delete socket path
    std::fs::remove_file(&socket_path).map_err(|e| {
        MachineError::FileRemoving(format!("fail to remove socket {}: {}", socket_path.display(), e.to_string()))
    })?;

    Ok(())
}
