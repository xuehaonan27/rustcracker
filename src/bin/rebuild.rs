use std::{
    sync::{Arc, Mutex}, process::Child,
};

use hyper::Response;
use rustfire::{
    client::firecracker_client::{
        create_sync_action, put_balloon, put_guest_boot_source, put_guest_drive_by_id,
        put_guest_network_interface_by_id, put_logger, put_machine_configuration,
        FirecrackerClient, launch, demo_boot_source, demo_machine_config, demo_rootfs, demo_start, demo_launch,
    },
    model::{
        balloon::Balloon,
        boot_source::BootSource,
        drive::Drive,
        instance_action_info::InstanceActionInfo,
        logger::{LogLevel, Logger},
        machine_configuration::MachineConfiguration,
        network_interface::NetworkInterface,
    },
};
use tokio::{sync::Notify, try_join, task::LocalSet};

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

#[tokio::main]
async fn main() -> Result<()> {
    let socket_path = "/tmp/firecracker.socket";
    let firecracker_binary_path = "/usr/bin/firecracker";
    let kernel_image_path = "/root/test_fire/vmlinux-5.10.198";
    let logger_path = "/root/test_fire/firecracker.log";
    let rootfs_path = "/root/test_fire/ubuntu-22.04.ext4";

    let client = FirecrackerClient::new(socket_path, firecracker_binary_path);
    // client.clear()?;
    // let child = client.launch()?;
    let client_arcmutex = Arc::new(Mutex::new(client));
    let notify = Arc::new(Notify::new());

    let local_set = LocalSet::new();


    // let client_arcmutex_clone1 = Arc::clone(&client_arcmutex);
    // let notify_clone1 = Arc::clone(&notify);
    // let task1 = tokio::task::spawn_local(async move {
    //     let instance = client_arcmutex_clone1.lock().unwrap();
    //     /* 忙等创建好套接字 */
    //     // while let Err(_) = fs::metadata(socket_path) {}
    //     while let Err(_) = fs::metadata(instance.get_socket_path()) {}
    //     notify_clone1.notify_waiters();
    // });

    let task1 = tokio::spawn(demo_launch(socket_path.into(), firecracker_binary_path.into(), 10));
    
    let child: Child = match try_join!(task1) {
        Ok((result,)) => {
            match result {
                Ok(child) => child,
                Err(_) => {
                    eprintln!("Fail when waiting socket");
                    panic!();
                },
            }
        },
        Err(_) => {
            eprintln!("Fail when waiting socket");
            panic!();
        },
    };

    /* 如果不进行firecracker_client的上锁的话那么就没问题了, 上锁过程应当仅仅对于socket进行, 而不是任何东西 */

    let boot_source = BootSource::default()
        .with_kernel_image_path(kernel_image_path)
        .with_boot_args("console=ttyS0 reboot=k panic=1 pci=off");
    let task_boot_source = tokio::spawn(demo_boot_source(socket_path.into(), boot_source));

    let rootfs = Drive::new()
        .with_drive_id("rootfs")
        .set_root_device(true)
        .set_read_only(false)
        .with_drive_path(rootfs_path);
    let task_rootfs = tokio::spawn(demo_rootfs(socket_path.into(), rootfs));

    let machine_config = MachineConfiguration::default()
        .with_vcpu_count(2)
        .with_mem_size_mib(1024)
        .set_hyperthreading(false)
        .set_track_dirty_pages(false);
    // let taskdemo = tokio::task::spawn_local(demo(Arc::clone(&client_arcmutex), Some(Arc::clone(&notify)), machine_config));
    let task_machine_config = tokio::spawn(demo_machine_config(socket_path.into(), machine_config));

    match tokio::try_join!(
        task_boot_source,
        task_machine_config,
        task_rootfs
    ) {
        Ok((result1, result2, result3)) => {
            match result1 {
                Ok(_) => (),
                Err(e) => eprintln!("Failed at putting boot source: {e}"),
            }
            match result2 {
                Ok(_) => (),
                Err(e) => eprintln!("Failed at putting root file system: {e}"),
            }
            match result3 {
                Ok(_) => (),
                Err(e) => eprintln!("Failed at putting machine config: {e}"),
            }
        },
        Err(err) => {
            eprintln!("At least one task failed: {err}");
        }
    }

    let task_spawn = tokio::spawn(demo_start(socket_path.into(), InstanceActionInfo::instance_start()));

    match try_join!(task_spawn) {
        Ok(_) => (),
        Err(e) => eprintln!("spawn error: {e}"),
    }

    let output = child.wait_with_output().unwrap();

    if !output.status.success() {
        eprintln!("virtual machine ran badly");
    }

    Ok(())
}
