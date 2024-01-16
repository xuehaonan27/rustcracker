use std::{path::PathBuf, pin::Pin, future::Future};

use nix::{fcntl, sys::stat::Mode, unistd};

use super::machine::Machine;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
#[allow(non_upper_case_globals)]
const StartVMMHandlerName: &'static str = "fcinit.StartVMM";
#[allow(non_upper_case_globals)]
const BootstrapLoggingHandlerName: &'static str = "fcinit.BootstrapLogging";
#[allow(non_upper_case_globals)]
pub(crate) const CreateLogFilesHandlerName: &'static str = "fcinit.CreateLogFilesHandler";
#[allow(non_upper_case_globals)]
const CreateMachineHandlerName: &'static str = "fcinit.CreateMachine";
#[allow(non_upper_case_globals)]
const CreateBootSourceHandlerName: &'static str = "fcinit.CreateBootSource";
#[allow(non_upper_case_globals)]
const AttachDrivesHandlerName: &'static str = "fcinit.AttachDrives";
#[allow(non_upper_case_globals)]
const CreateNetworkInterfacesHandlerName: &'static str = "fcinit.CreateNetworkInterfaces";
#[allow(non_upper_case_globals)]
const AddVsocksHandlerName: &'static str = "fcinit.AddVsocks";
#[allow(non_upper_case_globals)]
const SetMetadataHandlerName: &'static str = "fcinit.SetMetadata";
#[allow(non_upper_case_globals)]
const ConfigMmdsHandlerName: &'static str = "fcinit.ConfigMmds";
#[allow(non_upper_case_globals)]
pub(crate) const LinkFilesToRootFSHandlerName: &'static str = "fcinit.LinkFilesToRootFS";
#[allow(non_upper_case_globals)]
const SetupNetworkHandlerName: &'static str = "fcinit.SetupNetwork";
#[allow(non_upper_case_globals)]
const SetupKernelArgsHandlerName: &'static str = "fcinit.SetupKernelArgs";
#[allow(non_upper_case_globals)]
const CreateBalloonHandlerName: &'static str = "fcint.CreateBalloon";

#[allow(non_upper_case_globals)]
const ValidateCfgHandlerName: &'static str = "validate.Cfg";
#[allow(non_upper_case_globals)]
const ValidateJailerCfgHandlerName: &'static str = "validate.JailerCfg";
#[allow(non_upper_case_globals)]
const ValidateNetworkCfgHandlerName: &'static str = "validate.NetworkCfg";

pub struct Handler {
    pub(crate) name: &'static str,
    // pub(crate) func: fn(&mut Machine) -> Result<()>,
    pub(crate) func: Box<dyn Fn(&mut Machine) -> Result<()>>,
}

// pub struct AsyncHandler<'short, 'long> 
// where 'long: 'short
// {
//     pub(crate) name: &'static str,
//     // pub(crate) func: Box<dyn Fn(&mut Machine) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn Error>>>>> + Send>,
//     pub func: Box<dyn Fn(&'long mut Machine) -> Pin<Box<dyn Future<Output = Result<()>> + 'short>> + Send>,
// }

pub struct HandlerList(Vec<Handler>);
impl HandlerList {
    // append will append a new handler to the handler list.
    pub(crate) fn append(&mut self, handlers_vec: impl Into<Vec<Handler>>) {
        self.0.append(&mut handlers_vec.into());
    }

    // pub(crate) fn append_after(&mut self, name: String, handler: Handler) {
    //     let mut new_list: Vec<Handler> = Vec::new();
    //     self.0.into_iter().for_each(|h| {
    //         if h.name == name.as_str() {
    //             new_list.push(h);
    //             new_list.push(handler);
    //         } else {
    //             new_list.push(h);
    //         }
    //     });
    //     self.0 = new_list;
    // }

    // push will append a given handler after the specified handler.
    pub(crate) fn push(&mut self, handler: Handler) {
        self.0.push(handler);
    }

    // len return the length of the given handler list
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    // has will iterate through the handler list and check to see if the the named
    // handler exists.
    pub(crate) fn has(&self, name: &str) -> bool {
        for handler in &self.0 {
            if handler.name == name {
                return true
            }
        }
        false
    }

    // // swap will replace all elements of the given name with the new handler.
    // pub(crate) fn swap(&mut self, handler: Handler) {
    //     let mut new_list = Vec::new();
    //     self.0.iter().for_each(|h| {
    //         if h.name == handler.name {
    //             new_list.push(handler);
    //         } else {
    //             new_list.push(*h.clone());
    //         }
    //     });
    //     self.0 = new_list;
    // }

    // // swappend will either append, if there isn't an element within the handler
    // // list, otherwise it will replace all elements with the given name.
    // pub(crate) fn swappend(&mut self, handler: Handler) {
    //     if self.has(handler.name) {
    //         self.swap(handler);
    //     } else {
    //         self.push(handler);
    //     }
    // }

    // remove will return an updated handler with all instances of the specific
    // named handler being removed.
    pub(crate) fn remove(&mut self, name: String) {
        self.0.iter().filter(|&h| h.name != name);
    }

    // clear clears all named handler in the list.
    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }
}

#[derive(Clone)]
pub(crate) struct HandlersAdapter(
    pub(crate) fn(&Handlers) -> Result<()>,
);

pub struct Handlers {
    pub(crate) validation: HandlerList,
    pub(crate) fcinit: HandlerList,
}

// const DEFAULT_FCINIT_HANDLER_LIST: HandlerList = HandlerList(vec![
//     SetupNetworkHandler(),
//     SetupKernelArgsHandler(),
//     StartVMMHandler(),
//     CreateLogFilesHandler(),
//     BootstrapLoggingHandler(),
//     CreateMachineHandler(),
//     CreateBootSourceHandler(),
//     AttachDrivesHandler(),
//     CreateNetworkInterfacesHandler(),
//     AddVsocksHandler(),
// ]);

// const DEFAULT_VALIDATION_HANDLER_LIST: HandlerList = HandlerList(vec![
//     NetworkConfigValidationHandler(),
// ]);

// pub const DEFAULT_HANDLERS: Handlers = Handlers {
//     validation: DEFAULT_VALIDATION_HANDLER_LIST,
//     fcinit: DEFAULT_FCINIT_HANDLER_LIST,
// };

// ConfigValidationHandler is used to validate that required fields are
// present. This validator is to be used when the jailer is turned off.
#[allow(non_snake_case)]
pub fn ConfigValidationHandler() -> Handler { Handler {
    name: ValidateCfgHandlerName,
    func: Box::new(move |m: &mut Machine| -> Result<()> {
        m.cfg.validate()
    }),
}}

// async fn async_config_validation_func(m: &mut Machine) -> Result<()> {
//     Ok(())
// }

// #[allow(non_snake_case)]
// pub fn AsyncConfigValidationHandler<'a>() -> AsyncHandler<'a, 'a> {
//     AsyncHandler {
//         name: ValidateCfgHandlerName,
//         func: Box::new(|m: &mut Machine| Box::pin(async_config_validation_func(m))),
//     }
// }

// JailerConfigValidationHandler is used to validate that required fields are
// present.
#[allow(non_snake_case)]
pub fn JailerConfigValidationHandler() -> Handler { Handler {
    name: ValidateJailerCfgHandlerName,
    func: Box::new(move |m: &mut Machine| -> Result<()> {
        todo!()
    }),
}}

#[allow(non_snake_case)]
pub fn NetworkConfigValidationHandler() -> Handler { Handler {
    name: ValidateNetworkCfgHandlerName,
    func: Box::new(move |m: &mut Machine| -> Result<()> {
        todo!()
    }),
}}

// StartVMMHandler is a named handler that will handle starting of the VMM.
// This handler will also set the exit channel on completion.
#[allow(non_snake_case)]
pub fn StartVMMHandler() -> Handler { Handler {
    name: StartVMMHandlerName,
    func: Box::new(move |m: &mut Machine| -> Result<()> {
        todo!()
    }),
}}

fn create_fifo_or_file(
    machine: &mut Machine,
    fifo: Option<impl Into<PathBuf>>,
    path: Option<impl Into<PathBuf>>,
) -> Result<()> {
    if let Some(fifo) = fifo {
        unistd::mkfifo(&fifo.into(), Mode::S_IRUSR | Mode::S_IWUSR)?;
        Ok(())
    } else if let Some(path) = path {
        let raw_fd = fcntl::open(
            &path.into(),
            fcntl::OFlag::O_RDWR | fcntl::OFlag::O_CREAT | fcntl::OFlag::O_APPEND,
            Mode::S_IRUSR | Mode::S_IWUSR,
        )?;
        unistd::close(raw_fd);
        Ok(())
    } else {
        Err("create_fifo_or_file: parameters wrong".into())
    }
}

// CreateLogFilesHandler is a named handler that will create the fifo log files
#[allow(non_snake_case)]
pub fn CreateLogFilesHandler() -> Handler { Handler {
    name: CreateLogFilesHandlerName,
    func: Box::new(move |m: &mut Machine| -> Result<()> {
        create_fifo_or_file(m, m.cfg.metrics_fifo.clone(), m.cfg.metrics_path.clone())?;
        create_fifo_or_file(m, m.cfg.log_fifo.clone(), m.cfg.log_path.clone())?;
        if m.cfg.fifo_log_writer.is_some() {
            /* 重定向fifolog到这里 */
            todo!()
        }
        log::debug!("Createde metrics and logging fifos");

        Ok(())
    }),
}}

// BootstrapLoggingHandler is a named handler that will set up fifo logging of
// firecracker process.
#[allow(non_snake_case)]
pub fn BootstrapLoggingHandler() -> Handler { Handler {
    name: BootstrapLoggingHandlerName,
    func: Box::new(move |m: &mut Machine| -> Result<()> {
        todo!()
    }),
}}

// CreateMachineHandler is a named handler that will "create" the machine and
// upload any necessary configuration to the firecracker process.
#[allow(non_snake_case)]
pub fn CreateMachineHandler() -> Handler { Handler {
    name: CreateMachineHandlerName,
    func: Box::new(move |m: &mut Machine| -> Result<()> {
        todo!()
    }),
}}

// CreateBootSourceHandler is a named handler that will set up the booting
// process of the firecracker process.
#[allow(non_snake_case)]
pub fn CreateBootSourceHandler() -> Handler { Handler {
    name: CreateBootSourceHandlerName,
    func: Box::new(move |m: &mut Machine| -> Result<()> {
        todo!()
    }),
}}

// AttachDrivesHandler is a named handler that will attach all drives for the
// firecracker process.
#[allow(non_snake_case)]
pub fn AttachDrivesHandler() -> Handler { Handler {
    name: AttachDrivesHandlerName,
    func: Box::new(move |m: &mut Machine| -> Result<()> {
        todo!()
    }),
}}

// CreateNetworkInterfacesHandler is a named handler that registers network
// interfaces with the Firecracker VMM.
#[allow(non_snake_case)]
pub fn CreateNetworkInterfacesHandler() -> Handler { Handler {
    name: CreateNetworkInterfacesHandlerName,
    func: Box::new(move |m: &mut Machine| -> Result<()> {
        todo!()
    }),
}}

// SetupNetworkHandler is a named handler that will setup the network namespace
// and network interface configuration prior to the Firecracker VMM starting.
#[allow(non_snake_case)]
pub fn SetupNetworkHandler() -> Handler { Handler {
    name: SetupNetworkHandlerName,
    func: Box::new(move |m: &mut Machine| -> Result<()> {
        todo!()
    }),
}}

// SetupKernelArgsHandler is a named handler that will update any kernel boot
// args being provided to the VM based on the other configuration provided, if
// needed.
#[allow(non_snake_case)]
pub fn SetupKernelArgsHandler() -> Handler { Handler {
    name: SetupKernelArgsHandlerName,
    func: Box::new(move |m: &mut Machine| -> Result<()> {
        todo!()
    }),
}}

// AddVsocksHandler is a named handler that adds vsocks to the firecracker
// process.
#[allow(non_snake_case)]
pub fn AddVsocksHandler() -> Handler { Handler {
    name: AddVsocksHandlerName,
    func: Box::new(move |m: &mut Machine| -> Result<()> {
        todo!()
    }),
}}

// NewSetMetadataHandler is a named handler that puts the metadata into the
// firecracker process.
#[allow(non_snake_case)]
pub fn NewSetMetadataHandler(metadata: String) -> Handler {
    Handler {
        name: SetMetadataHandlerName,
        func: Box::new(move |m: &mut Machine| -> Result<()> {
            todo!()
        }),
    }
}

// ConfigMmdsHandler is a named handler that puts the MMDS config into the
// firecracker process.
#[allow(non_snake_case)]
pub fn ConfigMmdsHandler() -> Handler { Handler {
    name: ConfigMmdsHandlerName,
    func: Box::new(move |m: &mut Machine| -> Result<()> {
        todo!()
    }),
}}

// NewCreateBalloonHandler is a named handler that put a memory balloon into the
// firecracker process.
#[allow(non_snake_case)]
pub fn NewCreateBalloonHandler(amout_mib: u64, deflate_on_oon: bool, stats_polling_intervals: u64) -> Handler {
    Handler {
        name: CreateBalloonHandlerName,
        func: Box::new(move |m: &mut Machine| -> Result<()> {
            todo!()
        }),
    }
}