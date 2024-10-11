use lazy_static::lazy_static;
use rustcracker::{
    config::{HypervisorConfig, JailerConfig, MicroVMConfig},
    models::*,
};
use std::env;

lazy_static! {
    pub static ref JAILER_CONFIG: JailerConfig = JailerConfig {
        gid: env::var("JAILER_GID").ok().and_then(|s| s.parse().ok()),
        uid: env::var("JAILER_UID").ok().and_then(|s| s.parse().ok()),
        numa_node: None,
        exec_file: env::var("FRCK_BIN").ok(),
        jailer_bin: env::var("JAILER_BIN").ok(),
        chroot_base_dir: env::var("CHROOT_BASE_DIR").ok(),
        daemonize: Some(false),
    };
    pub static ref HYPERVISOR_NOJAILER_CONFIG: HypervisorConfig = HypervisorConfig {
        id: env::var("ID")
            .ok()
            .and_then(|s| Some(s + &uuid::Uuid::new_v4().to_string())),
        poll_status_secs: 20,
        launch_timeout: 5,
        using_jailer: Some(false),
        frck_bin: env::var("FRCK_BIN").ok(),
        jailer_bin: env::var("JAILER_BIN").ok(),
        jailer_config: None,
        frck_export_path: env::var("FRCK_EXPORT_PATH").ok(),
        socket_path: env::var("SOCKET_PATH").ok(),
        socket_retry: env::var("SOCKET_RETRY")
            .unwrap_or("3".to_string())
            .parse()
            .unwrap(),
        lock_path: env::var("LOCK_PATH").ok(),
        log_path: env::var("LOG_PATH").ok(),
        log_clear: Some(false),
        metrics_path: env::var("METRICS_PATH").ok(),
        metrics_clear: Some(false),
        network_clear: None,
        seccomp_level: None,
        // stdout_to: env::var("STDOUT_TO").ok(),
        // stderr_to: env::var("STDERR_TO").ok(),
        clear_jailer: env::var("CLEAR_JAILER")
            .ok()
            .and_then(|s| s.to_lowercase().parse().ok()),
    };
    pub static ref HYPERVISOR_WITHJAILER_CONFIG: HypervisorConfig = HypervisorConfig {
        id: env::var("ID")
            .ok()
            .and_then(|s| Some(s + &uuid::Uuid::new_v4().to_string())),
        poll_status_secs: 20,
        launch_timeout: 5,
        using_jailer: Some(true),
        frck_bin: env::var("FRCK_BIN").ok(),
        jailer_bin: env::var("JAILER_BIN").ok(),
        jailer_config: Some(JAILER_CONFIG.clone()),
        frck_export_path: env::var("FRCK_EXPORT_PATH").ok(),
        socket_path: env::var("SOCKET_PATH").ok(),
        socket_retry: env::var("SOCKET_RETRY")
            .unwrap_or("3".to_string())
            .parse()
            .unwrap(),
        lock_path: env::var("LOCK_PATH").ok(),
        log_path: env::var("LOG_PATH").ok(),
        log_clear: Some(false),
        metrics_path: env::var("METRICS_PATH").ok(),
        metrics_clear: Some(false),
        network_clear: None,
        seccomp_level: None,
        clear_jailer: env::var("CLEAR_JAILER")
            .ok()
            .and_then(|s| s.to_lowercase().parse().ok()),
    };
    pub static ref BOOT_SOURCE: BootSource = BootSource {
        boot_args: Some("console=ttyS0 reboot=k panic=1 pci=off".to_string()),
        initrd_path: None,
        kernel_image_path: env::var("KERNEL_IMAGE_PATH").expect("kernel image path not found"),
    };
    pub static ref ROOTFS: Drive = Drive {
        drive_id: "rootfs".to_string(),
        partuuid: None,
        is_root_device: true,
        cache_type: None,
        is_read_only: false,
        path_on_host: env::var("ROOTFS_PATH").expect("must set rootfs"),
        rate_limiter: None,
        io_engine: None,
        socket: None
    };
    pub static ref MICROVM_CONFIG: MicroVMConfig = MicroVMConfig {
        logger: Some(Logger {
            level: None,
            log_path: "firecracker.log".to_string(),
            show_level: None,
            show_log_origin: None,
            module: None
        }),
        metrics: None,
        boot_source: Some(BOOT_SOURCE.clone()),
        drives: Some(vec![ROOTFS.clone()]),
        network_interfaces: Some(vec![NetworkInterface {
            iface_id: "eth0".to_string(),
            guest_mac: None,
            host_dev_name: "tap0".to_string(),
            rx_rate_limiter: None,
            tx_rate_limiter: None,
        }]),
        vsock_devices: None,
        cpu_config: None,
        machine_config: Some(MachineConfiguration {
            cpu_template: None,
            ht_enabled: Some(false),
            mem_size_mib: 256,
            track_dirty_pages: None,
            vcpu_count: 4,
        }),
        vmid: Some("1234qwer".to_string()),
        net_ns: None,
        mmds_config: None,
        balloon: Some(Balloon {
            amount_mib: 64,
            deflate_on_oom: true,
            stats_polling_interval_s: None,
        }),
        entropy_device: None,
        init_metadata: Some("Hello there!".to_string()),
    };
}
