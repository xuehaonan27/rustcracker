// use rustfire::{client::firecracker_client::{demo_launch, demo_boot_source, demo_rootfs, demo_machine_config, demo_query_machine_config, demo_start}, model::{boot_source::BootSource, drive::Drive, machine_configuration::MachineConfiguration, instance_action_info::InstanceActionInfo}};
// use tokio::{task::JoinSet, try_join};


// type GenericError = Box<dyn std::error::Error + Send + Sync>;
// type Result<T> = std::result::Result<T, GenericError>;

// #[tokio::main]
// async fn main() -> Result<()> {
//     let socket_path = "/tmp/firecracker.socket";
//     let firecracker_binary_path = "/usr/bin/firecracker";
//     let kernel_image_path = "/root/test_fire/vmlinux-5.10.198";
//     // let logger_path = "/root/test_fire/firecracker.log";
//     let rootfs_path = "/root/test_fire/ubuntu-22.04.ext4";

//     let child = tokio::spawn(demo_launch(socket_path.into(), firecracker_binary_path.into(), 10)).await??;

//     let boot_source = BootSource::default()
//         .with_kernel_image_path(kernel_image_path)
//         .with_boot_args("console=ttyS0 reboot=k panic=1 pci=off");
//     let task_boot_source = tokio::spawn(demo_boot_source(socket_path.into(), boot_source));

//     let rootfs = Drive::new()
//         .with_drive_id("rootfs")
//         .set_root_device(true)
//         .set_read_only(false)
//         .with_drive_path(rootfs_path);
//     let task_rootfs = tokio::spawn(demo_rootfs(socket_path.into(), rootfs));

//     let machine_config = MachineConfiguration::default()
//         .with_vcpu_count(2)
//         .with_mem_size_mib(1024)
//         .set_hyperthreading(false)
//         .set_track_dirty_pages(false);
//     // let taskdemo = tokio::task::spawn_local(demo(Arc::clone(&client_arcmutex), Some(Arc::clone(&notify)), machine_config));
//     let task_machine_config = tokio::spawn(demo_machine_config(socket_path.into(), machine_config));

//     match try_join!(task_boot_source, task_rootfs, task_machine_config) {
//         Ok((_, _, _)) => (),
//         Err(_) => panic!("Sending failure"),
//     }
//     tokio::spawn(demo_start(socket_path.into(), InstanceActionInfo::instance_start())).await??;

//     const MAX_NUM: usize = 50000;
//     let mut set = JoinSet::new();

//     for i in 0..MAX_NUM {
//         set.spawn(demo_query_machine_config(socket_path.into(), i));
//     }
//     let mut seen = [false; MAX_NUM];
//     let mut cnt = 0;
//     while let Some(res) = set.join_next().await {
//         let result = res.unwrap();
//         match result {
//             Ok((_, i)) => {
//                 seen[i] = true;
//                 cnt += 1;
//             },
//             Err(_) => panic!("Fail at {}", cnt),
//         }
//     }

//     let output = child.wait_with_output().unwrap();

//     if !output.status.success() {
//         eprintln!("virtual machine ran badly");
//     }

//     // let fut = demo_machine_config(socket_path.into(), machine_config);

//     Ok(())
// }

fn main() {
    
}