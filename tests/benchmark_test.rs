use std::io::{BufRead, BufReader};

use log::error;
use rustcracker::{
    components::{
        command_builder::VMMCommandBuilder,
        machine::{Config, Machine, MachineError},
    },
    model::{
        cpu_template::{CPUTemplate, CPUTemplateString},
        drive::Drive,
        logger::LogLevel,
        machine_configuration::MachineConfiguration,
    },
    utils::TestArgs,
};
#[allow(unused)]
const NUMBER_OF_VMS: isize = 200;

#[allow(unused)]
fn create_machine(name: String, forward_signals: ()) -> Result<Machine, MachineError> {
    let dir = std::env::temp_dir().join(name);

    let socket_path = dir.join("api.sock");
    let vmlinux_path = TestArgs::test_data_path().join("./vmlinux");
    let log_fifo = dir.join("log.fifo");
    let metrics = dir.join("metrics.fifo");

    let config = Config {
        socket_path: Some(socket_path.to_owned()),
        kernel_image_path: Some(vmlinux_path),
        log_fifo: Some(log_fifo),
        metrics_fifo: Some(metrics),
        log_level: Some(LogLevel::Info),
        machine_cfg: Some(MachineConfiguration {
            vcpu_count: 1,
            cpu_template: Some(CPUTemplate(CPUTemplateString::T2)),
            mem_size_mib: 256,
            ht_enabled: Some(false),
            track_dirty_pages: None,
        }),
        drives: Some(vec![Drive {
            drive_id: "root".to_string(),
            is_root_device: true,
            is_read_only: true,
            path_on_host: TestArgs::test_root_fs(),
            partuuid: None,
            cache_type: None,
            rate_limiter: None,
            io_engine: None,
            socket: None,
        }]),
        ..Default::default()
    };

    let cmd = VMMCommandBuilder::new()
        .with_socket_path(&socket_path)
        .with_bin(&TestArgs::get_firecracker_binary_path())
        .build();
    let (_send, exit_recv) = async_channel::bounded(64);
    let (_send, sig_recv) = async_channel::bounded(64);
    let mut machine = Machine::new(config, exit_recv, sig_recv, 10, 60)?;
    machine.set_command(cmd.into());
    Ok(machine)
}

#[allow(unused)]
async fn start_and_wait_vm(m: &mut Machine) -> Result<(), MachineError> {
    m.start().await?;

    let log_fifo = m
        .get_log_file()
        .ok_or(MachineError::ArgWrong("no log file provided".to_string()))?;
    let file = std::fs::File::open(&log_fifo).map_err(|e| {
        error!(
            "fail to open file {}: {}",
            log_fifo.display(),
            e.to_string()
        );
        MachineError::FileAccess(format!(
            "fail to open file {}: {}",
            log_fifo.display(),
            e.to_string()
        ))
    })?;

    let buf_reader = BufReader::new(file);
    for line in buf_reader.lines() {
        let line = line.unwrap();
        if line.contains("Guest-boot-time") {
            break;
        }
    }

    m.stop_vmm().await?;
    m.wait().await?;

    Ok(())
}

