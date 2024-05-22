use crate::ops_res::{
    get_machine_configuration::{GetMachineConfigurationOps, GetMachineConfigurationRes},
    Operation, Response,
};

pub trait Event<O: Operation, R: Response> {
    fn new(ops: O) -> Self {
        let res = R::new();
        Self { ops, res }
    }

    fn get_ops(&self) -> &O;

    fn get_res(&self) -> &R;
}

pub struct GetMachineConfiguration {
    ops: GetMachineConfigurationOps,
    res: GetMachineConfigurationRes,
}

impl Event<GetMachineConfigurationOps, GetMachineConfigurationRes> for GetMachineConfiguration {
    fn get_ops(&self) -> &GetMachineConfigurationOps {
        &self.ops
    }

    fn get_res(&self) -> &GetMachineConfigurationRes {
        &self.res
    }
}
