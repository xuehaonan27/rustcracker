use crate::ops_res::{
    get_machine_configuration::{GetMachineConfigurationOps, GetMachineConfigurationRes},
    Operation, Response,
};

pub trait Event<O: Operation, R: Response> {
    // fn new(ops: O) -> Self;

    fn get_ops(&self) -> &O;

    fn get_res(&self) -> &R;

    fn get_res_mut(&mut self) -> &mut R;
}

pub struct GetMachineConfiguration {
    ops: GetMachineConfigurationOps,
    res: GetMachineConfigurationRes,
}

impl Event<GetMachineConfigurationOps, GetMachineConfigurationRes> for GetMachineConfiguration {
    // fn new(ops: GetMachineConfigurationOps) -> Self {
    //     Self { ops, res: GetMachineConfigurationRes::new() }
    // }

    fn get_ops(&self) -> &GetMachineConfigurationOps {
        &self.ops
    }

    fn get_res(&self) -> &GetMachineConfigurationRes {
        &self.res
    }

    fn get_res_mut(&mut self) -> &mut GetMachineConfigurationRes {
        &mut self.res
    }
}
