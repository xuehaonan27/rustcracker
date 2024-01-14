use rustfire::{
    client::{
        agent::{launch, wait_socket, clear},
        connection_pool::SocketConnectionPool,
    },
    model::{
        balloon::Balloon, boot_source::BootSource, drive::Drive,
        instance_action_info::InstanceActionInfo, machine_configuration::MachineConfiguration,
    }, utils::Json,
};


fn main() {
    let socket_path = "/tmp/firecracker.socket";
    let firecracker_binary_path = "/usr/bin/firecracker";
    let kernel_image_path = "/root/test_fire/vmlinux-5.10.198";
    // let logger_path = "/root/test_fire/firecracker.log";
    let rootfs_path = "/root/test_fire/ubuntu-22.04.ext4";
    clear(&socket_path.into()).expect("Could not delete");
    let child = launch(&firecracker_binary_path.into(), &socket_path.into())
        .expect("Fail to launch firecracker");
    wait_socket(&socket_path.into());
    
    let boot_source = BootSource::default()
        .with_kernel_image_path(kernel_image_path)
        .with_boot_args("console=ttyS0 reboot=k panic=1 pci=off");
    let rootfs = Drive::new()
        .with_drive_id("rootfs")
        .set_root_device(true)
        .set_read_only(false)
        .with_drive_path(rootfs_path);
    let machine_config = MachineConfiguration::default()
        .with_vcpu_count(2)
        .with_mem_size_mib(1024)
        .set_hyperthreading(false)
        .set_track_dirty_pages(false);
    let balloon = Balloon::new()
        .with_amount_mib(100)
        .with_stats_polling_interval_s(10)
        .set_deflate_on_oom(true);
    
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().worker_threads(8).build().unwrap();
    // use tokio::io::AsyncWriteExt;
    rt.spawn(async move {
        let cp = SocketConnectionPool::new(socket_path.into(), 8, 64, 8);

        cp.actor_handle.put_guest_boot_source(boot_source).await;
        // tokio::io::stdout().write_all(b"main: after put_guest_boot_source").await.unwrap();
        cp.actor_handle.put_drive(rootfs).await;
        // tokio::io::stdout().write_all(b"main: after put_guest_drive_by_id").await.unwrap();
        cp.actor_handle
            .put_machine_config(machine_config)
            .await;
        // tokio::io::stdout().write_all(b"main: after put_machine_configuration").await.unwrap();
        cp.actor_handle.put_balloon(balloon).await;
        // tokio::io::stdout().write_all(b"main: after put_balloon").await.unwrap();
        cp.actor_handle
        .create_sync_action(InstanceActionInfo::instance_start())
        .await;
        // tokio::io::stdout().write_all(b"main: after create_sync_action").await.unwrap();
        let balloon = cp.actor_handle.get_balloon().await;
        println!("Balloon: {}", balloon.to_json().unwrap());
    });
    
    let output = child.wait_with_output().unwrap();
    if !output.status.success() {
        eprintln!("virtual machine ran badly");
    }
}
