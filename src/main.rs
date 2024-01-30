// use std::{
//     sync::{Arc, Mutex}, process::Child,
// };

// use rustfire::{
//     client::firecracker_client::{
//         create_sync_action, put_balloon, put_guest_boot_source, put_guest_drive_by_id,
//         put_guest_network_interface_by_id, put_logger, put_machine_configuration,
//         FirecrackerClient, launch,
//     },
//     model::{
//         balloon::Balloon,
//         boot_source::BootSource,
//         drive::Drive,
//         instance_action_info::InstanceActionInfo,
//         logger::{LogLevel, Logger},
//         machine_configuration::MachineConfiguration,
//         network_interface::NetworkInterface,
//     },
// };
// use tokio::{sync::Notify, try_join, task::LocalSet};

// type GenericError = Box<dyn std::error::Error + Send + Sync>;
// type Result<T> = std::result::Result<T, GenericError>;

// #[tokio::main]
// async fn main() -> Result<()> {
//     let socket_path = "/tmp/firecracker.socket";
//     let firecracker_binary_path = "/usr/bin/firecracker";
//     let kernel_image_path = "/root/test_fire/vmlinux-5.10.198";
//     let logger_path = "/root/test_fire/firecracker.log";
//     let rootfs_path = "/root/test_fire/ubuntu-22.04.ext4";

//     let client = FirecrackerClient::new(socket_path, firecracker_binary_path);
//     // client.clear()?;
//     // let child = client.launch()?;
//     let client_arcmutex = Arc::new(Mutex::new(client));
//     let notify = Arc::new(Notify::new());

//     let local_set = LocalSet::new();

//     local_set.run_until(async move {

//     // let client_arcmutex_clone1 = Arc::clone(&client_arcmutex);
//     // let notify_clone1 = Arc::clone(&notify);
//     // let task1 = tokio::task::spawn_local(async move {
//     //     let instance = client_arcmutex_clone1.lock().unwrap();
//     //     /* 忙等创建好套接字 */
//     //     // while let Err(_) = fs::metadata(socket_path) {}
//     //     while let Err(_) = fs::metadata(instance.get_socket_path()) {}
//     //     notify_clone1.notify_waiters();
//     // });

//     let task1 = tokio::task::spawn_local(launch(Arc::clone(&client_arcmutex), Some(Arc::clone(&notify)), 10));
    
//     let child: Child = match try_join!(task1) {
//         Ok((result,)) => {
//             match result {
//                 Ok(child) => child,
//                 Err(_) => {
//                     eprintln!("Fail when waiting socket");
//                     panic!();
//                 },
//             }
//         },
//         Err(_) => {
//             eprintln!("Fail when waiting socket");
//             panic!();
//         },
//     };

//     // let client_arcmutex_clone2 = Arc::clone(&client_arcmutex);
//     // let notify_clone2 = Arc::clone(&notify);
//     // let task2 = tokio::task::spawn_local(async move {
//     //     let balloon: Balloon = Balloon::new()
//     //         .with_amount_mib(1024)
//     //         .with_stats_polling_interval_s(10)
//     //         .set_deflate_on_oom(true);
//     //     let instance = client_arcmutex_clone2.lock().unwrap();
//     //     notify_clone2.notified().await;
//     //     instance.put_balloon(balloon).await;
//     //     println!("rustfire:main[{}]: balloon put", line!());
//     // });

//     /* 如果不进行firecracker_client的上锁的话那么就没问题了, 上锁过程应当仅仅对于socket进行, 而不是任何东西 */
//     let balloon: Balloon = Balloon::new()
//         .with_amount_mib(1024)
//         .with_stats_polling_interval_s(10)
//         .set_deflate_on_oom(true);
//     let task_balloon = tokio::task::spawn_local(put_balloon(
//         Arc::clone(&client_arcmutex),
//         Some(Arc::clone(&notify)),
//         balloon,
//     ));

//     // let client_arcmutex_clone3 = Arc::clone(&client_arcmutex);
//     // let notify_clone3 = Arc::clone(&notify);
//     // let task3 = tokio::task::spawn_local(async move {
//     //     let boot_source = BootSource::default()
//     //         .with_kernel_image_path(kernel_image_path)
//     //         .with_boot_args("console=ttyS0 reboot=k panic=1 pci=off");
//     //     let instance = client_arcmutex_clone3.lock().unwrap();
//     //     notify_clone3.notified().await;
//     //     instance.put_guest_boot_source(boot_source).await;
//     //     println!("rustfire:main[{}]: boot source put", line!());
//     // });

//     let boot_source = BootSource::default()
//         .with_kernel_image_path(kernel_image_path)
//         .with_boot_args("console=ttyS0 reboot=k panic=1 pci=off");
//     let task_boot_source = tokio::task::spawn_local(put_guest_boot_source(
//         Arc::clone(&client_arcmutex),
//         Some(Arc::clone(&notify)),
//         boot_source,
//     ));

//     // let client_arcmutex_clone4 = Arc::clone(&client_arcmutex);
//     // let notify_clone4 = Arc::clone(&notify);
//     // let task4 = tokio::task::spawn_local(async move {
//     //     let rootfs = Drive::new()
//     //         .with_drive_id("rootfs")
//     //         .set_root_device(true)
//     //         .set_read_only(false)
//     //         .with_drive_path(rootfs_path);
//     //     let instance = client_arcmutex_clone4.lock().unwrap();
//     //     notify_clone4.notified().await;
//     //     instance.put_guest_drive_by_id(rootfs).await;
//     //     println!("rustfire:main[{}]: root file system put", line!());
//     // });

//     let rootfs = Drive::new()
//         .with_drive_id("rootfs")
//         .set_root_device(true)
//         .set_read_only(false)
//         .with_drive_path(rootfs_path);
//     let task_rootfs = tokio::task::spawn_local(put_guest_drive_by_id(
//         Arc::clone(&client_arcmutex),
//         Some(Arc::clone(&notify)),
//         rootfs,
//     ));

//     // let client_arcmutex_clone5 = Arc::clone(&client_arcmutex);
//     // let notify_clone5 = Arc::clone(&notify);
//     // let task5 = tokio::task::spawn_local(async move {
//     //     let machine_config = MachineConfiguration::default()
//     //         .with_vcpu_count(2)
//     //         .with_mem_size_mib(1024)
//     //         .set_hyperthreading(false)
//     //         .set_track_dirty_pages(false);
//     //     let instance = client_arcmutex_clone5.lock().unwrap();
//     //     notify_clone5.notified();
//     //     instance.put_machine_configuration(machine_config).await;
//     //     println!(
//     //         "rustfire:main[{}]: machine configuration specified",
//     //         line!()
//     //     );
//     // });

//     let machine_config = MachineConfiguration::default()
//         .with_vcpu_count(2)
//         .with_mem_size_mib(1024)
//         .set_hyperthreading(false)
//         .set_track_dirty_pages(false);
//     // let taskdemo = tokio::task::spawn_local(demo(Arc::clone(&client_arcmutex), Some(Arc::clone(&notify)), machine_config));
//     let task_machine_config = tokio::task::spawn_local(put_machine_configuration(
//         Arc::clone(&client_arcmutex),
//         Some(Arc::clone(&notify)),
//         machine_config,
//     ));

//     // let client_arcmutex_clone6 = Arc::clone(&client_arcmutex);
//     // let notify_clone6 = Arc::clone(&notify);
//     // let task6 = tokio::task::spawn_local(async move {
//     //     let logger = Logger::default()
//     //         .with_log_path(logger_path)
//     //         .with_log_level(LogLevel::Warning)
//     //         .set_show_level(true)
//     //         .set_show_origin(true);
//     //     let instance = client_arcmutex_clone6.lock().unwrap();
//     //     notify_clone6.notified();
//     //     instance.put_logger(logger).await;
//     //     println!("rustfire:main[{}]: logger put", line!());
//     // });

//     let logger = Logger::default()
//         .with_log_path(logger_path)
//         .with_log_level(LogLevel::Warning)
//         .set_show_level(true)
//         .set_show_origin(true);
//     let task_logger = tokio::task::spawn_local(put_logger(
//         Arc::clone(&client_arcmutex),
//         Some(Arc::clone(&notify)),
//         logger,
//     ));

//     // let client_arcmutex_clone7 = Arc::clone(&client_arcmutex);
//     // let notify_clone7 = Arc::clone(&notify);
//     // let task7 = tokio::task::spawn_local(async move {
//     //     let network_interface = NetworkInterface::default()
//     //         .with_host_dev_name("/var/run/netns/my_netns")
//     //         .with_iface_id("Someidhere");
//     //     let instance = client_arcmutex_clone7.lock().unwrap();
//     //     notify_clone7.notified();
//     //     instance
//     //         .put_guest_network_interface_by_id(network_interface)
//     //         .await;
//     //     println!("rustfire:main[{}]: network interface set", line!());
//     // });

//     let network_interface = NetworkInterface::default()
//         .with_host_dev_name("/var/run/netns/my_netns")
//         .with_iface_id("Someidhere");
//     let task_network_interface = tokio::task::spawn_local(put_guest_network_interface_by_id(
//         Arc::clone(&client_arcmutex),
//         Some(Arc::clone(&notify)),
//         network_interface,
//     ));

//     // tokio::join!(task1, task2, task3, task4, task5, task6, task7);

//     match tokio::try_join!(
//         task_balloon,
//         task_boot_source,
//         task_logger,
//         task_machine_config,
//         task_network_interface,
//         task_rootfs
//     ) {
//         Ok((result2, result3, result4, result5, result6, result7)) => {
//             match result2 {
//                 Ok(_) => (),
//                 Err(e) => eprintln!("Failed at putting balloon: {e}"),
//             }
            
//             match result3 {
//                 Ok(_) => (),
//                 Err(e) => eprintln!("Failed at putting boot source: {e}"),
//             }

//             match result4 {
//                 Ok(_) => (),
//                 Err(e) => eprintln!("Failed at putting logger: {e}"),
//             }

//             match result5 {
//                 Ok(_) => (),
//                 Err(e) => eprintln!("Failed at putting machine config: {e}"),
//             }

//             match result6 {
//                 Ok(_) => (),
//                 Err(e) => eprintln!("Failed at putting network interface: {e}"),
//             }

//             match result7 {
//                 Ok(_) => (),
//                 Err(e) => eprintln!("Failed at putting root file system: {e}"),
//             }
//         },
//         Err(err) => {
//             eprintln!("At least one task failed: {err}");
//         }
//     }

//     // client_arcmutex.lock().unwrap()
//     //     .create_sync_action(InstanceActionInfo::instance_start())
//     //     .await?;
//     // println!(
//     //     "rustfire:main[{}]: virtual machine instance started",
//     //     line!()
//     // );

//     let task_spawn = tokio::task::spawn_local(create_sync_action(
//         Arc::clone(&client_arcmutex),
//         Some(Arc::clone(&notify)),
//         InstanceActionInfo::instance_start(),
//     ));

//     match try_join!(task_spawn) {
//         Ok(_) => (),
//         Err(e) => eprintln!("spawn error: {e}"),
//     }

//     let output = child.wait_with_output().unwrap();

//     if !output.status.success() {
//         eprintln!("virtual machine ran badly");
//     }

//     }).await;

//     Ok(())
// }
use std::error::Error;
use hyper::{body::HttpBody, Client};
use hyperlocal::{UnixClientExt, Uri};
use rustfire::{client::{command_builder::VMMCommandBuilder, machine::{test_utils::start_vmm, Config, Machine, MachineError}}, model::{cpu_template::{CPUTemplate, CPUTemplateString}, machine_configuration::MachineConfiguration}, utils::{check_kvm, init, make_socket_path, TestArgs}};
use tokio::io::{self, AsyncWriteExt as _};
use log::{debug, info};

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
//     let url = Uri::new("/tmp/hyperlocal.sock", "/").into();

//     let client = Client::unix();

//     let mut response = client.get(url).await?;

//     while let Some(next) = response.data().await {
//         let chunk = next?;
//         io::stdout().write_all(&chunk).await?;
//     }

//     Ok(())
// }

fn main() {
    test_start_once().expect("fail");
}

fn test_start_vmm() -> Result<(), MachineError> {
    init();
    check_kvm()?;
    let socket_path = make_socket_path("test_start_vmm");

    let cfg = Config {
        socket_path: Some(socket_path.to_owned()),
        machine_cfg: Some(MachineConfiguration::default()),
        ..Default::default()
    };

    let cmd = VMMCommandBuilder::new().with_socket_path(&socket_path).with_bin(&TestArgs::get_firecracker_binary_path()).build();
    
    let (exit_send, exit_recv) = async_channel::bounded(64);
    let (sig_send, sig_recv) = async_channel::bounded(64);
    let mut m = Machine::new(cfg, exit_recv, sig_recv, 10, 60)?;
    m.set_command(cmd.into());

    m.clear_validation();

    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        MachineError::Initialize("fail to create tokio runtime".to_string())
    })?;

    let (send, recv) = tokio::sync::oneshot::channel();
    rt.block_on(async move {
        tokio::select! {
            output = start_vmm(&mut m) => {
                send.send(output).unwrap();
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(250)) => {
                info!("firecracker ran for 250ms and still start_vmm doesn't return");
                m.stop_vmm().await.expect("fail to stop_vmm!");
            }
        };

        // stop_vmm force anyway
        _ = m.stop_vmm_force();
    });

    let msg = recv.blocking_recv().unwrap();
    info!("start_vmm sent: {:#?}", msg);

    // close channels
    exit_send.close();
    sig_send.close();

    // delete socket path
    std::fs::remove_file(&socket_path).map_err(|e| {
        MachineError::FileRemoving(format!("fail to remove socket {}: {}", socket_path.display(), e.to_string()))
    })?;

    Ok(())
}

fn test_start_once() -> Result<(), MachineError> {
    init();
    check_kvm()?;

    let socket_path = make_socket_path("test_start_one");

    let cfg = Config {
        socket_path: Some(socket_path.to_owned()),
        disable_validation: true,
        kernel_image_path: Some(TestArgs::get_vmlinux_path()?),
        machine_cfg: Some(MachineConfiguration{
            vcpu_count: 1,
            mem_size_mib: 64,
            cpu_template: Some(CPUTemplate(CPUTemplateString::T2)),
            ht_enabled: Some(false),
            track_dirty_pages: None,
        }),
        ..Default::default()
    };

    let cmd = VMMCommandBuilder::new().with_socket_path(&socket_path).with_bin(&TestArgs::get_firecracker_binary_path()).build();
    let (exit_send, exit_recv) = async_channel::bounded(64);
    let (sig_send, sig_recv) = async_channel::bounded(64);
    let mut m = Machine::new(cfg, exit_recv, sig_recv, 10, 60)?;
    m.set_command(cmd.into());

    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        MachineError::Initialize("fail to create tokio runtime".to_string())
    })?;

    let (send1, recv1) = tokio::sync::oneshot::channel();
    let (send2, recv2) = tokio::sync::oneshot::channel();

    rt.block_on(async move {
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

        debug!("calling stop_vmm_force");
        _ = m.stop_vmm_force().await;
    });

    let msg1 = recv1.blocking_recv().unwrap();
    info!("start1 sent: {:#?}", msg1);
    let msg2 = recv2.blocking_recv().unwrap();
    info!("start2 sent: {:#?}", msg2);

    // close channels
    exit_send.close();
    sig_send.close();

    // delete socket path
    std::fs::remove_file(&socket_path).map_err(|e| {
        MachineError::FileRemoving(format!("fail to remove socket {}: {}", socket_path.display(), e.to_string()))
    })?;

    Ok(())
}
