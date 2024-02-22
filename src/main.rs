use rustcracker::{components::{command_builder::VMMCommandBuilder, machine::{Config, Machine, MachineError}}, model::{cpu_template::{CPUTemplate, CPUTemplateString}, machine_configuration::MachineConfiguration}, utils::{check_kvm, init, make_socket_path, TestArgs}};
use log::{debug, info};

fn main() {
    test_start_vmm().expect("fail");
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
    
    let mut m = Machine::new(cfg)?;
    m.set_command(cmd.into());

    let rt = tokio::runtime::Runtime::new().map_err(|_e| {
        MachineError::Initialize("fail to create tokio runtime".to_string())
    })?;

    let (send, recv) = tokio::sync::oneshot::channel();
    rt.block_on(async move {
        tokio::select! {
            output = m.start_vmm_test() => {
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
    // let (sig_send, sig_recv) = async_channel::bounded(64);
    let mut m = Machine::new(cfg)?;
    m.set_command(cmd.into());

    let rt = tokio::runtime::Runtime::new().map_err(|_e| {
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

    // delete socket path
    std::fs::remove_file(&socket_path).map_err(|e| {
        MachineError::FileRemoving(format!("fail to remove socket {}: {}", socket_path.display(), e.to_string()))
    })?;

    Ok(())
}
