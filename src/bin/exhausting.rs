use std::path::PathBuf;

use log::{error, info};
use run_script::ScriptOptions;
use rustcracker::{
    components::{
        command_builder::VMMCommandBuilder,
        machine::{Config, Machine, MachineError, MachineMessage},
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
use tokio::task::JoinSet;

// directory that hold all the runtime structures.
const RUN_DIR: &'static str = "/tmp/rustcracker/run";

// directory that holds resources e.g. kernel image and file system image.
const RESOURCE_DIR: &'static str = "/tmp/rustcracker/res";

// directory that holds snapshots
const SNAPSHOT_DIR: &'static str = "/tmp/rustcracker/snapshot";

// directory that holds the legacy of machines
async fn run(id: usize) -> Result<(), MachineError> {
    // Initialize env logger
    let _ = env_logger::builder().is_test(false).try_init();

    // check that kvm is accessible
    check_kvm()?;

    /* below are configurations that could be transmitted with json file (Serializable and Deserializable) */

    /* ############ configurations begin ############ */
    // the name of this microVM
    let vmid = format!("name{id}");

    // the directory that holds this microVM
    // /tmp/rustfire/run/name{id}
    let dir = PathBuf::from(RUN_DIR).join(vmid.to_owned());
    std::fs::create_dir_all(&dir).map_err(|e| {
        MachineError::FileCreation(format!(
            "fail to create {}: {}",
            dir.display(),
            e.to_string()
        ))
    })?;

    // suppose that the logger is going to be created at "${RUN_DIR}/logger"
    // /tmp/rustfire/run/name{id}/log.fifo
    let log_fifo = dir.join(format!("log{id}.fifo"));

    // metrics path
    // /tmp/rustcracker/run/name{id}/metrics.fifo
    let metrics_fifo = dir.join(format!("metrics{id}.fifo"));

    // unix domain socket (communicate with firecracker) path
    // /tmp/rustcracker/run/name{id}/api.sock
    let socket_path = dir.join("api.sock");

    // kernel image path (prepare valid kernel image here)
    // /tmp/rustcracker/res/vmlinux
    let vmlinux_path = PathBuf::from(&RESOURCE_DIR).join("vmlinux");

    // root fs path (prepare valid root file system image here)
    // /tmp/rustcracker/res/rootfs
    let rootfs_path = PathBuf::from(&RESOURCE_DIR).join("rootfs");

    // firecracker binary
    // /tmp/rustcracker/res/firecracker
    let firecracker_path = PathBuf::from(&RESOURCE_DIR).join("firecracker");

    // path that holds snapshot
    let snapshot_dir = PathBuf::from(&SNAPSHOT_DIR).join(vmid.to_owned());
    std::fs::create_dir_all(&snapshot_dir).map_err(|e| {
        MachineError::FileCreation(format!(
            "fail to create {}: {}",
            snapshot_dir.display(),
            e.to_string()
        ))
    })?;
    let snapshot_mem = snapshot_dir.join(format!("mem{id}"));
    let snapshot_path = snapshot_dir.join(format!("snapshot{id}"));

    let init_metadata = r#"{
        "name": "Xue Haonan",
        "email": "xuehaonan27@gmail.com"
    }"#;

    // write the configuration of the firecraker process
    let config = Config {
        // microVM's name
        vmid: Some(vmid),
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
        // kernel_args might be overrided
        // if any network interfaces configured, the `ip` field may be added or modified
        kernel_args: Some("".to_string()),
        network_interfaces: Some(vec![NetworkInterface {
            guest_mac: Some("06:00:AC:10:00:02".to_string()),
            host_dev_name: format!("tap{id}").into(),
            iface_id: "net1".into(),
            rx_rate_limiter: None,
            tx_rate_limiter: None,
        }]),
        net_ns: Some("my_netns".into()),
        balloon: Some(
            Balloon::new()
                .with_amount_mib(100)
                .with_stats_polling_interval_s(5)
                .set_deflate_on_oom(true),
        ),
        init_metadata: Some(init_metadata.to_string()),
        fifo_log_writer: None,
        // configurations that could be set yourself and I don't want to set here
        forward_signals: None,
        log_path: None,
        metrics_path: None,
        initrd_path: None,

        // virtio devices
        vsock_devices: None,
        // when running in production environment, don't set this true to avoid validation
        disable_validation: false,
        jailer_cfg: None,
        seccomp_level: None,
        mmds_address: None,
        stdin: Some(StdioTypes::Null),
        stdout: Some(StdioTypes::From {
            path: log_fifo.to_owned(),
        }),
        stderr: Some(StdioTypes::From {
            path: log_fifo.to_owned(),
        }),
        log_clear: Some(true),
        metrics_clear: Some(true),
        network_clear: Some(true),
    };
    /* ############ configurations end ############ */

    /* ############ Launching microVM ############ */
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
    // ${firecracker_path} --api-sock ${socket_path} --id ${config.vmid}
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
    }"#
    .to_string();
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
    // one should always pause the microVM before trying to create snapshot for it
    machine.pause().await?;
    info!(target: "Pause", "Paused");
    machine.create_snapshot(snapshot_mem, snapshot_path).await?;
    machine.resume().await?;
    info!(target: "Resume", "Resumed");

    /* ############ Exiting microVM ############ */
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

#[tokio::main]
async fn main() -> Result<(), MachineError> {
    info!(target: "Main", "preparing...");

    let mut set = JoinSet::new();
    for id in 0..10 {
        // run the shell script to config networking first
        // to run shell script dirctly in rust I chose crate `run_script`
        // although such behavior might not be the best practice
        // and the script it self is fetched directly from the doc of `firecracker`
        // with minor modification to config networking from name0 to name9
        let (code, _output, error) = run_script::run_script!(
            r#"
            TAP_DEV="tap$1"
            HOST_IFACE="eth$1"
            TAP_IP="172.16.0.1"
            MASK_SHORT="/30"

            # Setup network interface
            sudo ip link del "$TAP_DEV" 2> /dev/null || true
            sudo ip tuntap add dev "$TAP_DEV" mode tap
            sudo ip addr add "${TAP_IP}${MASK_SHORT}" dev "$TAP_DEV"
            sudo ip link set dev "$TAP_DEV" up

            # Enable ip forwarding
            sudo sh -c "echo 1 > /proc/sys/net/ipv4/ip_forward"

            # Set up microVM internet access
            sudo iptables -t nat -D POSTROUTING -o "$HOST_IFACE" -j MASQUERADE || true
            sudo iptables -D FORWARD -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT \
                || true
            sudo iptables -D FORWARD -i "$TAP_DEV" -o "$HOST_IFACE" -j ACCEPT || true
            sudo iptables -t nat -A POSTROUTING -o "$HOST_IFACE" -j MASQUERADE
            sudo iptables -I FORWARD 1 -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT
            sudo iptables -I FORWARD 1 -i "$TAP_DEV" -o "$HOST_IFACE" -j ACCEPT
            "#,
            &vec![format!("{id}")],
            &ScriptOptions::new()
        ).unwrap();
        // if networking is configured successfully, then run the machine `name{id}``
        if code == 0 && error == "" {
            set.spawn(run(id));
        }
    }
    while let Some(a) = set.join_next().await {
        a.unwrap()?;
    }

    Ok(())
}

/*
next step:
1. add optional clearing files
2. add env var override
*/