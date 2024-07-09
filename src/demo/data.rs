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
        id: env::var("ID").ok(),
        numa_node: None,
        exec_file: env::var("FRCK_BIN").ok(),
        jailer_bin: env::var("JAILER_BIN").ok(),
        chroot_base_dir: env::var("CHROOT_BASE_DIR").ok(),
        daemonize: Some(false),
    };
    pub static ref HYPERVISOR_NOJAILER_CONFIG: HypervisorConfig = HypervisorConfig {
        launch_timeout: 5,
        using_jailer: Some(false),
        frck_bin: env::var("FRCK_BIN").ok(),
        jailer_bin: env::var("JAILER_BIN").ok(),
        jailer_config: None,
        frck_export_path: env::var("FRCK_EXPORT_PATH").ok(),
        socket_path: env::var("socket_path").ok(),
        lock_path: env::var("LOCK_PATH").ok(),
        log_path: env::var("LOG_PATH").ok(),
        log_clear: Some(false),
        metrics_path: env::var("METRICS_PATH").ok(),
        metrics_clear: Some(false),
        network_clear: None,
        seccomp_level: None,
    };
    pub static ref HYPERVISOR_WITHJAILER_CONFIG: HypervisorConfig = HypervisorConfig {
        launch_timeout: 5,
        using_jailer: Some(true),
        frck_bin: env::var("FRCK_BIN").ok(),
        jailer_bin: env::var("JAILER_BIN").ok(),
        jailer_config: Some(JAILER_CONFIG.clone()),
        frck_export_path: env::var("FRCK_EXPORT_PATH").ok(),
        socket_path: env::var("socket_path").ok(),
        lock_path: env::var("LOCK_PATH").ok(),
        log_path: env::var("LOG_PATH").ok(),
        log_clear: Some(false),
        metrics_path: env::var("METRICS_PATH").ok(),
        metrics_clear: Some(false),
        network_clear: None,
        seccomp_level: None,
    };
    pub static ref BOOT_SOURCE: BootSource = BootSource {
        boot_args: Some("console=ttyS0 reboot=k panic=1 pci=off".to_string()),
        initrd_path: None,
        kernel_image_path: env::var("KERNEL_IMAGE_PATH").expect("kernel image path not found"),
    };
    pub static ref MICROVM_CONFIG: MicroVMConfig = MicroVMConfig {
        logger: None,
        metrics: None,
        boot_source: Some(BOOT_SOURCE.clone()),
        drives: None,
        network_interfaces: None,
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
        mmds_address: None,
        balloon: Some(Balloon {
            amount_mib: 64,
            deflate_on_oom: true,
            stats_polling_interval_s: None,
        }),
        entropy_device: None,
        init_metadata: Some("Hello there!".to_string()),
    };
}
