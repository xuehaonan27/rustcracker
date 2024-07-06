// use std::path::Path;

// use rustcracker::{
//     events::{
//         events::{self, Event},
//         events_async::{self, EventAsync},
//     },
//     models::{
//         boot_source::BootSource,
//         drive::Drive,
//         instance_action_info::{ActionType, InstanceActionInfo},
//         logger::{LogLevel, Logger},
//         machine_configuration::MachineConfiguration,
//         network_interface::NetworkInterface,
//     },
//     pressure_test,
//     rtck::Rtck,
//     rtck_async::RtckAsync,
//     RtckResult,
// };

// fn main() {
//     let mode = std::env::args()
//         .skip(1)
//         .next()
//         .expect("Need to specify sync/async/test");
//     let socket = std::env::args()
//         .skip(2)
//         .next()
//         .expect("Need to specify socket address");
//     println!("Launching in {} mode at {}", mode, socket);
//     match mode.as_str() {
//         "sync" => {
//             let _ = sync_main(socket).map_err(|e| panic!("async main error: {e}"));
//         }
//         #[cfg(feature = "tokio")]
//         "async" => {
//             let rt = tokio::runtime::Runtime::new().expect("Fail to create tokio runtime");
//             let _ = rt
//                 .block_on(async { async_main(socket).await })
//                 .map_err(|e| panic!("async main error: {e}"));
//         }
//         "test" => {
//             pressure_test(socket.parse::<usize>().expect("Fail to get write times"));
//         }
//         _ => panic!("Need to specify sync/async"),
//     }
// }

// fn sync_main<P: AsRef<Path>>(socket: P) -> RtckResult<()> {
//     let stream = bufstream::BufStream::new(std::os::unix::net::UnixStream::connect(socket)?);

//     let mut rtck = Rtck::from_stream(stream);

//     let mut put_machine_config = events::PutMachineConfiguration::new(MachineConfiguration {
//         cpu_template: None,
//         ht_enabled: None,
//         mem_size_mib: 256,
//         track_dirty_pages: None,
//         vcpu_count: 8,
//     });

//     rtck.execute(&mut put_machine_config)?;

//     if put_machine_config.is_succ() {
//         println!("Put machine configuration succeeded");
//     } else {
//         eprintln!("Put machine configuration error")
//     }

//     let mut get_machine_config = events::GetMachineConfiguration::new();

//     rtck.execute(&mut get_machine_config)?;

//     if get_machine_config.is_succ() {
//         println!("Get machine configuration succeeded");
//     } else {
//         eprintln!("Get machine configuration error")
//     }

//     Ok(())
// }

// async fn async_main<P: AsRef<Path>>(socket: P) -> RtckResult<()> {
//     // Select a stream that implements AsyncBufRead and AsyncWrite traits
//     // Connect the stream to target unix socket
//     let stream = tokio::io::BufStream::new(tokio::net::UnixStream::connect(socket).await?);

//     // Create an asynchronous Rtck with this stream
//     let mut rtck = RtckAsync::from_stream(stream);

//     let put_logger = events_async::PutLogger::new(Logger {
//         level: Some(LogLevel::Debug),
//         log_path: "~/test_fire/firecracker.log".into(),
//         show_level: Some(true),
//         show_log_origin: Some(true),
//         module: None,
//     });

//     rtck.execute(&put_logger).await?;

//     let put_guest_boot_source = events_async::PutGuestBootSource::new(BootSource {
//         kernel_image_path: "./vmlinux-5.10.217".to_string(),
//         boot_args: Some("console=ttyS0 reboot=k panic=1 pci=off".to_string()),
//         initrd_path: None,
//     });

//     rtck.execute(&put_guest_boot_source).await?;

//     let put_guest_drive_by_id = events_async::PutGuestDriveById::new(Drive {
//         drive_id: "rootfs".to_string(),
//         path_on_host: "./ubuntu-22.04.ext4".to_string(),
//         is_read_only: false,
//         is_root_device: true,
//         partuuid: None,
//         cache_type: None,
//         rate_limiter: None,
//         io_engine: None,
//         socket: None,
//     });

//     rtck.execute(&put_guest_drive_by_id).await?;

//     let put_guest_network_interface_by_id =
//         events_async::PutGuestNetworkInterfaceById::new(NetworkInterface {
//             guest_mac: Some("06:00:AC:10:00:02".to_string()),
//             host_dev_name: "tap0".to_string(),
//             iface_id: "net1".to_string(),
//             rx_rate_limiter: None,
//             tx_rate_limiter: None,
//         });

//     rtck.execute(&put_guest_network_interface_by_id).await?;

//     // Create an asynchronous event
//     let put_machine_config = events_async::PutMachineConfiguration::new(MachineConfiguration {
//         cpu_template: None,
//         ht_enabled: None,
//         mem_size_mib: 256,
//         track_dirty_pages: None,
//         vcpu_count: 8,
//     });

//     // Execute this event with the Rtck
//     rtck.execute(&put_machine_config).await?;

//     // Inspect the status of the event
//     if put_machine_config.is_succ() {
//         println!("Put machine configuration succeeded");
//     } else {
//         eprintln!("Put machine configuration error")
//     }

//     let get_machine_config = events_async::GetMachineConfiguration::new();

//     rtck.execute(&get_machine_config).await?;

//     if get_machine_config.is_succ() {
//         println!("Get machine configuration succeeded");
//     } else {
//         eprintln!("Get machine configuration error")
//     }

//     // Start the machine
//     let start_machine = events_async::CreateSyncAction::new(InstanceActionInfo {
//         action_type: ActionType::InstanceStart,
//     });

//     rtck.execute(&start_machine).await?;

//     Ok(())
// }

// async fn _demo_use_async_machine() -> RtckResult<()> {
//     use rustcracker::config::GlobalConfig;
//     let config = GlobalConfig {
//         ..Default::default()
//     };

//     use rustcracker::machine::machine_async;
//     use rustcracker::models::snapshot_create_params;
//     let machine = machine_async::Machine::create(&config).await?;
//     machine.configure().await?;
//     machine.start().await?;
//     machine.pause().await?;
//     machine
//         .snapshot(
//             "/snapshot/state/demo",
//             "/snapshot/mem/demo",
//             snapshot_create_params::SnapshotType::Diff,
//         )
//         .await?;
//     machine.resume().await?;
//     machine.stop().await?;
//     machine.delete().await?;
//     machine.delete_and_clean().await?;

//     Ok(())
// }

fn main() {
    
}