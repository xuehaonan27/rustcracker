use rustfire::{
    client::machine::{Config, Machine},
    model::{
        cpu_template::{self, CPUTemplate},
        machine_configuration::MachineConfiguration,
    },
};

const FIRECRACKER_BINARY_PATH: &'static str = "firecracker";
const FIRECRACKER_BINARY_OVERRIDE_ENV: &'static str = "FC_TEST_BIN";
const DEFAULT_JAILER_BINARY: &'static str = "jailer";
const JAILER_BINARY_OVERRIDE_ENV: &'static str = "FC_TEST_JAILER_BIN";
const DEFUALT_TUNTAP_NAME: &'static str = "fc-test-tap0";
const TUNTAP_OVERRIDE_ENV: &'static str = "FC_TEST_TAP";
const TEST_DATA_PATH_ENV: &'static str = "FC_TEST_DATA_PATH";
const SUDO_UID: &'static str = "SUDO_UID";
const SUDO_GID: &'static str = "SUDO_GID";

#[test]
fn test_new_machine() {
    let config: Config = Config::default().with_machine_config(
        MachineConfiguration::default()
            .with_vcpu_count(1)
            .with_mem_size_mib(100)
            .with_cpu_template(&CPUTemplate(cpu_template::CPUTemplateString::T2))
            .set_hyperthreading(false)
    ).set_disable_validation(true);
    let m = Machine::new(config);
    let m = match m {
        Ok(m) => m,
        Err(e) => panic!("failed to create new machine: {}", e),
    };
}
