use std::path::PathBuf;
use log::{error, info};
use nix::{fcntl::OFlag, sys::stat::Mode};
use rustcracker::{components::{jailer::JailerConfig, machine::{Config, Machine, MachineError, MachineMessage}}, model::{balloon::Balloon, cpu_template::{CPUTemplate, CPUTemplateString}, drive::Drive, logger::LogLevel, machine_configuration::MachineConfiguration, network_interface::NetworkInterface}, utils::{check_kvm, StdioTypes}};

// directory that hold all the runtime structures.
const RUN_DIR: &'static str = "/tmp/rustcracker/run";

// directory that holds resources e.g. kernel image and file system image.
const RESOURCE_DIR: &'static str = "/tmp/rustcracker/res";

// a tokio coroutine that might be parallel with other coroutines
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize env logger
    let _ = env_logger::builder().is_test(false).try_init();

    // check that kvm is accessible
    check_kvm()?;

    /* below are configurations that could be transmitted with json file (Serializable and Deserializable) */

    /* ############ configurations begin ############ */
    let jailer_uid = 123;
    let jailer_gid = 100;

    // /tmp/rustcracker/run/jailer
    let chroot_dir = PathBuf::from(&RUN_DIR).join("jailer");
    std::fs::create_dir_all(&chroot_dir).map_err(|e| {
        MachineError::FileCreation(format!(
            "fail to create dir path {}: {}",
            chroot_dir.display(),
            e.to_string()
        ))
    })?;

    // kernel image path (prepare valid kernel image here)
    // /tmp/rustcracker/res/vmlinux
    let vmlinux_path = PathBuf::from(&RESOURCE_DIR).join("vmlinux");

    // root fs path (prepare valid root file system image here)
    // /tmp/rustcracker/res/rootfs
    let rootfs_path = PathBuf::from(&RESOURCE_DIR).join("rootfs");

    // firecracker binary
    // /tmp/rustcracker/res/firecracker
    let firecracker_path = PathBuf::from(&RESOURCE_DIR).join("firecracker");

    // jailer binary
    // /tmp/rustcracker/res/jailer
    let jailer_path = PathBuf::from(&RESOURCE_DIR).join("jailer");

    let init_metadata = r#"{
        "name": "Xue Haonan",
        "email": "xuehaonan27@gmail.com"
    }"#;

    // the name of this microVM
    let vmid = "name2";

    // path that holds snapshot
    let snapshot_mem: PathBuf = "mem".into();
    let snapshot_path: PathBuf = "snapshot".into();
    
    let socket_path = "api.sock";
    let log_fifo = chroot_dir.join("firecracker.log");
    let metrics_fifo = chroot_dir.join("firecracker-metrics");
    let captured_log = chroot_dir.join("writer.fifo");

    // local logger file
    // ./log/benchmark_jailer
    let log_path: PathBuf = "logs/benchmark_jailer".into();
    std::fs::create_dir_all(&log_path).map_err(|e| {
        MachineError::FileCreation(format!("fail to create {}: {}", log_path.display(), e.to_string()))
    })?;
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

    let config = Config {
        // microVM's name
        vmid: Some(vmid.to_string()),
        // the path to unix domain socket which is relative to {chroot_dir}/firecracker/{vmid}/root/
        socket_path: Some(socket_path.into()),
        kernel_image_path: Some(vmlinux_path),
        log_fifo: Some(log_fifo.to_owned()),
        metrics_fifo: Some(metrics_fifo.to_owned()),
        log_level: Some(LogLevel::Debug),
        // the configuration of the microVM
        machine_cfg: Some(MachineConfiguration {
            // give microVM 1 virtual CPU
            vcpu_count: 2,
            // config correct CPU template here (as same as physical CPU template)
            cpu_template: Some(CPUTemplate(CPUTemplateString::None)),
            // give microVM 256 MiB memory
            mem_size_mib: 256,
            // disable hyperthreading
            ht_enabled: Some(false),
            track_dirty_pages: None,
        }),
        drives: Some(vec![Drive {
            // name that you like
            drive_id: "root".to_string(),
            // root fs is the ONLY root device that should be configured
            is_root_device: true,
            // if set true, then user cannot write to the rootfs
            is_read_only: true,
            path_on_host: rootfs_path,
            partuuid: None,
            cache_type: None,
            rate_limiter: None,
            io_engine: None,
            socket: None,
        }]),
        // jailer configuration
        jailer_cfg: Some(JailerConfig {
            jailer_binary: Some(jailer_path),
            gid: Some(jailer_gid),
            uid: Some(jailer_uid),
            numa_node: Some(0),
            id: Some(vmid.to_string()),
            chroot_base_dir: Some(chroot_dir),
            exec_file: Some(firecracker_path),
            stdout: Some(StdioTypes::FromRawFd { fd: log_fd }),
            stderr: Some(StdioTypes::FromRawFd { fd: log_fd }),
            daemonize: Some(false),
            stdin: None,
        }),
        // kernel_args might be overrided
        // if any network interfaces configured, the `ip` field may be added or modified
        kernel_args: Some("".to_string()),
        network_interfaces: Some(vec![NetworkInterface {
            guest_mac: Some("06:00:AC:10:00:02".to_string()),
            host_dev_name: "tap0".into(),
            iface_id: "net1".into(),
            rx_rate_limiter: None,
            tx_rate_limiter: None,
        }]),
        fifo_log_writer: Some(captured_log),
        net_ns: None,
        init_metadata: Some(init_metadata.to_string()),
        // configurations that could be set yourself and I don't want to set here
        forward_signals: None,
        log_path: None,
        metrics_path: None,
        initrd_path: None,
        vsock_devices: None,
        disable_validation: true,
        seccomp_level: None,
        mmds_address: None,
        balloon: Some(
            Balloon::new()
                .with_amount_mib(100)
                .with_stats_polling_interval_s(5)
                .set_deflate_on_oom(true),
        ),
        stdout: None,
        stderr: None,
        stdin: None,
        log_clear: Some(true),
        metrics_clear: Some(true),
        network_clear: Some(true),
    };
    /* ############ configurations end ############ */

    // use exit_send to send a force stop instruction (MachineMessage::StopVMM) to the microVM
    let (exit_send, exit_recv) = async_channel::bounded(64);
    // use sig_send to send a signal to firecracker process (yet implemented)
    let (sig_send, sig_recv) = async_channel::bounded(64);
    let mut machine = Machine::new(config, exit_recv, sig_recv, 10, 60)?;

    // command is already built by jailer

    // start the microVM
    machine.start().await.map_err(|e| {
        // remove the socket, log and metrics in case we start fail
        let _ = std::fs::remove_file(&socket_path);
        let _ = std::fs::remove_file(&log_fifo);
        let _ = std::fs::remove_file(&metrics_fifo);
        e
    })?;

    /* ############ Checking microVM ############ */
    let metadata = machine.get_metadata().await?;
    info!(target: "Metadata", "{}", metadata);

    let instance_info = machine.describe_instance_info().await?;
    info!(target: "InstanceInfo", "{:#?}", instance_info);

    let balloon = machine.get_balloon_config().await?;
    info!(target: "Balloon", "{:#?}", balloon);

    let balloon_stats = machine.get_balloon_stats().await?;
    info!(target: "BalloonStats", "{:#?}", balloon_stats);

    /* ############ Modifying microVM ############ */
    let new_metadata = r#"{
        "name":"Mugen_Cyaegha",
        "email":"897657514@qq.com"
    }"#.to_string();
    machine.update_metadata(&new_metadata).await?;
    // machine.update_balloon(10).await?;
    // machine.update_balloon_stats(3).await?;
    machine.refresh_machine_configuration().await?;

    /* ############ Checking microVM ############ */
    let metadata = machine.get_metadata().await?;
    info!(target: "Re-Metadata", "{}", metadata);

    let instance_info = machine.describe_instance_info().await?;
    info!(target: "Re-InstanceInfo", "{:#?}", instance_info);

    let balloon = machine.get_balloon_config().await?;
    info!(target: "Re-Balloon", "{:#?}", balloon);

    let balloon_stats = machine.get_balloon_stats().await?;
    info!(target: "Re-BalloonStats", "{:#?}", balloon_stats);

    /* ############ Saving microVM ############ */
    machine.pause().await?;
    info!(target: "Pause", "Paused");
    machine.create_snapshot(snapshot_mem, snapshot_path).await?;
    machine.resume().await?;
    info!(target: "Resume", "Resumed");

    // wait for the machine to exit.
    // Machine::wait will block until the firecracker process exit itself
    // or explicitly send it a exit message through exit_send defined previously
    // so spawn a isolated tokio task to wait for the machine.
    async fn timer(
        send: async_channel::Sender<MachineMessage>,
        secs: u64,
    ) -> Result<(), MachineError> {
        tokio::time::sleep(tokio::time::Duration::from_secs(secs)).await;
        send.send(MachineMessage::StopVMM).await.map_err(|e| {
            error!(target: "benchmark::timer", "error when sending a exit message: {}", e);
            send.close();
            MachineError::Execute(format!(
                "error when sending a exit message: {}",
                e.to_string()
            ))
        })?;
        send.close();
        Ok(())
    }

    // set a timer to send exit message to firecracker after 10 seconds
    tokio::spawn(timer(exit_send, 10));
    machine.wait().await?;

    // close the channel
    sig_send.close();

    Ok(())
}