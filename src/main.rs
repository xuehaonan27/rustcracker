use rustcracker::{
    models::machine_configuration::MachineConfiguration, ops_res::{
        get_machine_configuration::{GetMachineConfigurationOperation, GetMachineConfigurationResponse},
        put_machine_configuration::{PutMachineConfigurationOperation, PutMachineConfigurationResponse},
    }, rtck::Rtck, rtck_async::RtckAsync, RtckResult
};

fn main() {
    sync_main().expect("sync main error");
}

fn sync_main() -> RtckResult<()> {
    let stream = bufstream::BufStream::new(std::os::unix::net::UnixStream::connect("/tmp/api.sock")?);

    let mut rtck = Rtck::from_stream(stream);

    let put_machine_config = PutMachineConfigurationOperation::new(MachineConfiguration {
        cpu_template: None,
        ht_enabled: None,
        mem_size_mib: 256,
        track_dirty_pages: None,
        vcpu_count: 8,
    });

    rtck.send_request(&put_machine_config)?;
    rtck.recv_response::<PutMachineConfigurationResponse>()?;

    let get_machine_config = GetMachineConfigurationOperation::new();
    rtck.send_request(&get_machine_config)?;
    rtck.recv_response::<GetMachineConfigurationResponse>()?;

    todo!()
}

/// Event
pub struct PutMachineConfiguration {

}

/// Async event
pub struct PutMachineConfigurationAsync {

}

async fn async_main() -> RtckResult<()> {
    let stream = tokio::io::BufStream::new(tokio::net::UnixStream::connect("/tmp/api.sock").await?);

    let mut rtck = RtckAsync::from_stream(stream);

    let put_machine_config = PutMachineConfigurationOperation::new(MachineConfiguration {
        cpu_template: None,
        ht_enabled: None,
        mem_size_mib: 256,
        track_dirty_pages: None,
        vcpu_count: 8,
    });

    // Cooperative.
    rtck.send_request(&put_machine_config).await?;
    rtck.recv_response::<PutMachineConfigurationResponse>().await?;

    let get_machine_config = GetMachineConfigurationOperation::new();
    rtck.send_request(&get_machine_config).await?;
    rtck.recv_response::<GetMachineConfigurationResponse>().await?;
    todo!()
}

// Implement bursty version. Add queuing and timeout to Rtcks.