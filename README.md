# DEPRECATED: This crate is no longer maintained. Use [firecracker-rs-sdk](https://github.com/xuehaonan27/firecracker-rs-sdk) instead.

# rustcracker
A crate for communicating with [firecracker](https://github.com/firecracker-microvm/firecracker) developed by [Xue Haonan](https://github.com/xuehaonan27) during development of [PKU-cloud](https://github.com/lcpu-club/PKU-cloud). Reference: [firecracker-go-sdk](https://github.com/gbionescu/firecracker-go-sdk)

Thanks for supports from all members of LCPU (Linux Club of Peking University).


# Break Changes
The API of rustcracker 2.0.0 has a break change, which is completely incompatible with 1.x, and is cleaner, more organized and easier to use.

# Prepare Your Environment
* Get firecracker from [firecracker's getting-started page](https://github.com/firecracker-microvm/firecracker/blob/main/docs/getting-started.md)
* If you fail to get kernel image or rootfs image from [firecracker's getting-started page](https://github.com/firecracker-microvm/firecracker/blob/main/docs/getting-started.md), you could:
    * Build vmlinux image and rootfs image on your own.
    * Try these instead, which are also provided by Amazon AWS: 
        * kernel image: https://s3.amazonaws.com/spec.ccfc.min/img/hello/kernel/hello-vmlinux.bin
        * rootfs: https://s3.amazonaws.com/spec.ccfc.min/img/hello/fsfiles/hello-rootfs.ext4

* Get rustcracker:
    * From [crates.io](https://crates.io/crates/rustcracker)
    * From source code:
        ```bash
        git clone https://github.com/xuehaonan27/rustcracker
        cd rustcracker
        cargo build
        ```

# Example
```rust
// You should pass in hypervisor configuration to create a hypervisor.
// Then a microVM configuration to start a firecracker microVM instance/

async fn using() {
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

    hypervisor.stop().await.expect("fail to stop");
    log::info!("microVM stopped");
    sleep(3).await;

    // `delete` will collect resources with RAII mechanism.
    hypervisor.delete().await.expect("fail to delete");
    log::info!("microVM deleted");
}


// Provide a hypervisor configuration
let HYPERVISOR_WITHJAILER_CONFIG: HypervisorConfig = HypervisorConfig {
    // id max length is 64
    id: Some("demo-instance".to_string()),
    poll_status_secs: 20,
    launch_timeout: 5,
    using_jailer: Some(true),
    frck_bin: Some("/usr/bin/firecracker".to_string()),
    jailer_bin: Some("/usr/bin/jailer".to_string()),
    jailer_config: Some(JAILER_CONFIG.clone()),
    frck_export_path: None,
    socket_path: Some("run/firecracker.socket".to_string()),
    socket_retry: 3,
    lock_path: Some("run/firecracker.lock".to_string()),
    log_path: Some("var/log/firecracker.log".to_string()),
    log_clear: Some(false),
    metrics_path: None,
    metrics_clear: Some(false),
    network_clear: None,
    seccomp_level: None,
    stdout_to: None,
    stderr_to: None,
    clear_jailer: Some(false),
};

// Provide a microVM configuration
let MICROVM_CONFIG: MicroVMConfig = MicroVMConfig {
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
    mmds_address: None,
    balloon: Some(Balloon {
        amount_mib: 64,
        deflate_on_oom: true,
        stats_polling_interval_s: None,
    }),
    entropy_device: None,
    init_metadata: Some("Hello there!".to_string()),
};

```