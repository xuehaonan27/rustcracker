# rustcracker
A crate for communicating with [firecracker](https://github.com/firecracker-microvm/firecracker) developed by [Xue Haonan](https://github.com/xuehaonan27) during development of [PKU-cloud](https://github.com/lcpu-club/PKU-cloud). Reference: [firecracker-go-sdk](https://github.com/gbionescu/firecracker-go-sdk)

Thanks for supports from all members of LCPU (Linux Club of Peking University).

# Example
```Rust
async fn _demo_use_async_machine() -> RtckResult<()> {
    use rustcracker::config::GlobalConfig;
    let config = GlobalConfig {
        ..Default::default()
    };

    use rustcracker::machine::machine_async;
    use rustcracker::models::snapshot_create_params;
    let machine = machine_async::Machine::create(&config).await?;
    machine.configure().await?;
    machine.start().await?;
    machine.pause().await?;
    machine
        .snapshot(
            "/snapshot/state/demo",
            "/snapshot/mem/demo",
            snapshot_create_params::SnapshotType::Diff,
        )
        .await?;
    machine.resume().await?;
    machine.stop().await?;
    machine.delete().await?;
    machine.delete_and_clean().await?;

    Ok(())
}
```