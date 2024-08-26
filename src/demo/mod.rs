use data::{HYPERVISOR_NOJAILER_CONFIG, HYPERVISOR_WITHJAILER_CONFIG, MICROVM_CONFIG};
use rustcracker::{
    hypervisor::Hypervisor,
    options::{HypervisorOptions, MicroVMOptions},
};

pub mod data;

async fn sleep(secs: u64) {
    tokio::time::sleep(tokio::time::Duration::from_secs(secs)).await
}

#[allow(unused)]
async fn demo() {
    // read hypervisor
    dotenvy::dotenv().ok();
    let mut hypervisor = Hypervisor::new(&HYPERVISOR_WITHJAILER_CONFIG)
        .await
        .expect("fail to create hypervisor");
    // check remote
    hypervisor.ping_remote().await.expect("fail to ping remote");
    // start microVM
    hypervisor
        .start(&MICROVM_CONFIG)
        .await
        .expect("fail to configure microVM");
    // pause microVM
    hypervisor.pause().await.expect("fail to pause");
    // resume microVM
    hypervisor.resume().await.expect("fail to resume");
    // stop microVM (cannot recover)
    hypervisor.stop().await.expect("fail to stop");
    // stop firecracker, releasing resources with RAII
    hypervisor.delete().await.expect("fail to delete");
}

pub async fn no_jailer() {
    dotenvy::dotenv().ok();

    let mut hypervisor = Hypervisor::new(&HYPERVISOR_NOJAILER_CONFIG)
        .await
        .expect("fail to create hypervisor");

    sleep(3).await;

    hypervisor.ping_remote().await.expect("fail to ping remote");

    sleep(3).await;

    hypervisor
        .start(&MICROVM_CONFIG)
        .await
        .expect("fail to configure microVM");

    sleep(3).await;

    hypervisor.pause().await.expect("fail to pause");

    sleep(3).await;

    hypervisor.resume().await.expect("fail to resume");

    sleep(3).await;

    hypervisor.stop().await.expect("fail to stop");
}

pub async fn with_jailer() {
    dotenvy::dotenv().ok();

    let mut hypervisor = Hypervisor::new(&HYPERVISOR_WITHJAILER_CONFIG)
        .await
        .expect("fail to create hypervisor");
    log::info!("Hypervisor created");
    sleep(3).await;

    hypervisor.ping_remote().await.expect("fail to ping remote");
    log::info!("Hypervisor running!");
    sleep(3).await;

    hypervisor
        .start(&MICROVM_CONFIG)
        .await
        .expect("fail to configure microVM");
    log::info!("microVM configured");
    sleep(3).await;

    hypervisor.pause().await.expect("fail to pause");
    log::info!("microVM paused");
    sleep(3).await;

    hypervisor.resume().await.expect("fail to resume");
    log::info!("microVM resumed");
    sleep(3).await;

    hypervisor.stop().await.expect("fail to stop");
    log::info!("microVM stopped");
    sleep(3).await;

    hypervisor.delete().await.expect("fail to delete");
    log::info!("microVM deleted");
}

pub async fn using() {
    dotenvy::dotenv().ok();

    let mut hypervisor = Hypervisor::new(&HYPERVISOR_WITHJAILER_CONFIG)
        .await
        .expect("fail to create hypervisor");
    log::info!("Hypervisor created");
    sleep(3).await;

    hypervisor.ping_remote().await.expect("fail to ping remote");
    log::info!("Hypervisor running!");
    sleep(3).await;

    hypervisor
        .start(&MICROVM_CONFIG)
        .await
        .expect("fail to configure microVM");
    log::info!("microVM configured");
    sleep(3).await;

    let _ = hypervisor.wait().await;

    hypervisor.delete().await.expect("fail to delete");
    log::info!("microVM deleted");
}

pub async fn force_terminating() {
    dotenvy::dotenv().ok();

    let mut hypervisor = Hypervisor::new(&HYPERVISOR_WITHJAILER_CONFIG)
        .await
        .expect("fail to create hypervisor");
    log::info!("Hypervisor created");
    sleep(3).await;

    hypervisor.ping_remote().await.expect("fail to ping remote");
    log::info!("Hypervisor running!");
    sleep(3).await;

    hypervisor
        .start(&MICROVM_CONFIG)
        .await
        .expect("fail to configure microVM");
    log::info!("microVM configured");
    sleep(3).await;

    sleep(30).await;

    hypervisor.stop().await.expect("fail to stop");

    hypervisor.delete().await.expect("fail to delete");
    log::info!("microVM deleted");
}

pub async fn reusing_hypervisor() {
    dotenvy::dotenv().ok();

    let mut hypervisor = Hypervisor::new(&HYPERVISOR_WITHJAILER_CONFIG)
        .await
        .expect("fail to create hypervisor");

    sleep(3).await;

    hypervisor.ping_remote().await.expect("fail to ping remote");

    sleep(3).await;

    hypervisor
        .start(&MICROVM_CONFIG)
        .await
        .expect("fail to configure microVM");

    sleep(3).await;

    hypervisor.pause().await.expect("fail to pause");

    sleep(3).await;

    hypervisor.resume().await.expect("fail to resume");

    sleep(3).await;

    hypervisor
        .start(&MICROVM_CONFIG)
        .await
        .expect("fail to configure microVM");

    sleep(3).await;

    hypervisor.pause().await.expect("fail to pause");

    sleep(3).await;

    hypervisor.resume().await.expect("fail to resume");

    sleep(3).await;

    hypervisor
        .start(&MICROVM_CONFIG)
        .await
        .expect("fail to configure microVM");

    sleep(3).await;

    hypervisor.pause().await.expect("fail to pause");

    sleep(3).await;

    hypervisor.resume().await.expect("fail to resume");

    sleep(3).await;

    hypervisor.stop().await.expect("fail to stop");

    hypervisor.delete().await.expect("fail to delete");
}

pub fn syncusing() {
    use rustcracker::sync_hypervisor::Hypervisor;
    dotenvy::dotenv().ok();

    let mut hypervisor =
        Hypervisor::new(&HYPERVISOR_WITHJAILER_CONFIG).expect("fail to create hypervisor");
    log::info!("Hypervisor created");
    std::thread::sleep(std::time::Duration::from_secs(3));

    hypervisor.ping_remote().expect("fail to ping remote");
    log::info!("Hypervisor running!");
    std::thread::sleep(std::time::Duration::from_secs(3));

    hypervisor
        .start(&MICROVM_CONFIG)
        .expect("fail to configure microVM");
    log::info!("microVM configured");
    std::thread::sleep(std::time::Duration::from_secs(3));

    let _ = hypervisor.wait();

    hypervisor.delete().expect("fail to delete");
    log::info!("microVM deleted");
}

pub async fn options() {
    let options = HypervisorOptions::new()
        .using_jailer(true)
        .id("instance-demo-test")
        .poll_status_secs(20)
        .launch_timeout(5)
        .frck_bin("/root/firecracker")
        .jailer_bin("/root/jailer")
        .socket_path("/run/firecracker.socket")
        .socket_retry(3)
        .lock_path("/run/firecracker.lock")
        .log_path("/var/log/firecracker.log")
        .log_clear(false)
        .metrics_path("/var/metrics/firecracker.metrics")
        .metrics_clear(false)
        .clear_jailer(false)
        .jailer_gid(10000)
        .jailer_uid(10123)
        .chroot_base_dir("/srv/jailer")
        .daemonize(false)
        .validate()
        .expect("Invalid options");

    let mut hypervisor = options.spawn().await.expect("fail to create hypervisor");
    log::info!("Hypervisor created");

    hypervisor.ping_remote().await.expect("fail to ping remote");
    log::info!("Hypervisor running!");

    use rustcracker::models::*;
    let logger = Logger {
        level: None,
        log_path: "firecracker.log".to_string(),
        show_level: None,
        show_log_origin: None,
        module: None,
    };
    let boot_source = BootSource {
        boot_args: Some("console=ttyS0 reboot=k panic=1 pci=off".to_string()),
        initrd_path: None,
        kernel_image_path: "/root/kernel/hello-vmlinux.bin".to_string(),
    };
    let rootfs = Drive {
        drive_id: "rootfs".to_string(),
        partuuid: None,
        is_root_device: true,
        cache_type: None,
        is_read_only: false,
        path_on_host: "/root/drives/ubuntu.img".to_string(),
        rate_limiter: None,
        io_engine: None,
        socket: None,
    };
    let network_interface = NetworkInterface {
        iface_id: "eth0".to_string(),
        guest_mac: None,
        host_dev_name: "tap0".to_string(),
        rx_rate_limiter: None,
        tx_rate_limiter: None,
    };
    let machine_config = MachineConfiguration {
        cpu_template: None,
        ht_enabled: Some(false),
        mem_size_mib: 256,
        track_dirty_pages: None,
        vcpu_count: 4,
    };
    let vmid = "1234qwer";
    let balloon = Balloon {
        amount_mib: 64,
        deflate_on_oom: true,
        stats_polling_interval_s: None,
    };
    let init_metadata = "Hello, world!";

    let microvm = MicroVMOptions::new()
        .logger(logger)
        .boot_source(boot_source)
        .drives(vec![rootfs])
        .network_interfaces(vec![network_interface])
        .machine_config(machine_config)
        .vmid(vmid)
        .balloon(balloon)
        .init_metadata(init_metadata);

    microvm
        .instance(&mut hypervisor)
        .await
        .expect("fail to configure microVM");
    log::info!("microVM configured");

    let _ = hypervisor.wait().await;

    hypervisor.delete().await.expect("fail to delete");
    log::info!("microVM deleted");
}
