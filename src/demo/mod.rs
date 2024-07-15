use data::{HYPERVISOR_NOJAILER_CONFIG, HYPERVISOR_WITHJAILER_CONFIG, MICROVM_CONFIG};
use rustcracker::hypervisor::Hypervisor;

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
