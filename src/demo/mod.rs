use data::{HYPERVISOR_NOJAILER_CONFIG, HYPERVISOR_WITHJAILER_CONFIG, MICROVM_CONFIG};
use rustcracker::hypervisor::Hypervisor;

pub mod data;

async fn sleep(secs: u64) {
    tokio::time::sleep(tokio::time::Duration::from_secs(secs)).await
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
