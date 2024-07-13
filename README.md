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

    hypervisor.delete().await.expect("fail to delete");
    log::info!("microVM deleted");
}
```