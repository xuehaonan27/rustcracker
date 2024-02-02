use std::path::PathBuf;

use log::error;
use rustcracker::{
    components::{
        command_builder::VMMCommandBuilder,
        machine::{Config, Machine, MachineError, MachineMessage},
        // network::{StaticNetworkConfiguration, UniNetworkInterface, UniNetworkInterfaces},
    },
    model::{
        balloon::Balloon,
        cpu_template::{CPUTemplate, CPUTemplateString},
        drive::Drive,
        logger::LogLevel,
        machine_configuration::MachineConfiguration,
        network_interface::NetworkInterface,
    },
    utils::{check_kvm, StdioTypes},
};

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
    // the name of this microVM
    let vmid = "name1";

    // the directory that holds this microVM
    // /tmp/rustfire/run/name1
    let dir = PathBuf::from(RUN_DIR).join(vmid);

    // suppose that the logger is going to be created at "${RUN_DIR}/logger"
    // /tmp/rustfire/run/name1/log.fifo
    let log_fifo = dir.join("log.fifo");

    // metrics path
    // /tmp/rustcracker/run/name1/metrics.fifo
    let metrics_fifo = dir.join("metrics.fifo");

    // unix domain socket (communicate with firecracker) path
    // /tmp/rustcracker/run/name1/api.sock
    let socket_path = dir.join("api.sock");

    // kernel image path (prepare valid kernel image here)
    // /tmp/rustcracker/res/vmlinux
    let vmlinux_path = PathBuf::from(&RESOURCE_DIR).join("vmlinux");

    // root fs path (prepare valid root file system image here)
    // /tmp/rustcracker/res/rootfs
    let rootfs_path = PathBuf::from(&RESOURCE_DIR).join("rootfs");

    // firecracker binary
    // /tmp/rustcracker/res
    let firecracker_path = PathBuf::from(&RESOURCE_DIR).join("firecracker");

    // get the output of child
    // /tmp/rustcracker/run/name1/output.log
    let output_path = PathBuf::from("/tmp/output.log");

    let init_metadata = r#"{
        "name": "Xue Haonan",
        "email": "xuehaonan27@gmail.com"
    }"#;

    // write the configuration of the firecraker process
    let config = Config {
        // microVM's name
        vmid: Some(vmid.to_string()),
        // the path to unix domain socket that you want the firecracker to spawn
        socket_path: Some(socket_path.to_owned()),
        kernel_image_path: Some(vmlinux_path),
        log_fifo: Some(log_fifo.to_owned()),
        metrics_fifo: Some(metrics_fifo.to_owned()),
        log_level: Some(LogLevel::Debug),
        // the configuration of the microVM
        machine_cfg: Some(MachineConfiguration {
            // give microVM 1 virtual CPU
            vcpu_count: 1,
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
        // configurations that could be set yourself and I don't want to set here
        forward_signals: None,
        log_path: None,
        metrics_path: None,
        initrd_path: None,
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
        fifo_log_writer: Some(output_path),
        // virtio devices
        vsock_devices: None,
        // when running in production environment, don't set this true to avoid validation
        disable_validation: false,
        jailer_cfg: None,
        net_ns: Some("my_netns".into()),
        seccomp_level: None,
        mmds_address: None,
        balloon: Some(
            Balloon::new()
                .with_amount_mib(100)
                .with_stats_polling_interval_s(5)
                .set_deflate_on_oom(true),
        ),
        init_metadata: Some(init_metadata.to_string()),
        stdin: Some(StdioTypes::Null),
        stdout: Some(StdioTypes::From { path: log_fifo.to_owned() }),
        stderr: Some(StdioTypes::From { path: log_fifo.to_owned() }),
    };
    /* ############ configurations end ############ */

    // validate the config

    // use exit_send to send a force stop instruction (MachineMessage::StopVMM) to the microVM
    let (exit_send, exit_recv) = async_channel::bounded(64);
    // use sig_send to send a signal to firecracker process (yet implemented)
    let (sig_send, sig_recv) = async_channel::bounded(64);
    let mut machine = Machine::new(config, exit_recv, sig_recv, 10, 60)?;

    // build your own microVM command
    let cmd = VMMCommandBuilder::new()
        .with_socket_path(&socket_path)
        .with_bin(&firecracker_path)
        .build();

    // set your own microVM command (optional)
    // if not, then the machine will start using default command
    // ${firecracker_path} --api-sock ${socket_path} --seccomp-level 0 --id ${config.vmid}
    // (seccomp level 0 means disable seccomp)
    machine.set_command(cmd.into());

    // start the microVM
    machine.start().await.map_err(|e| {
        // remove the socket, log and metrics in case we start fail
        let _ = std::fs::remove_file(&socket_path);
        let _ = std::fs::remove_file(&log_fifo);
        let _ = std::fs::remove_file(&metrics_fifo);
        e
    })?;

    // ask for the metadata stored.

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
