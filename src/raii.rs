use crate::{RtckError, RtckResult};
use nix::unistd::Pid;
use std::path::PathBuf;

#[cfg(any(target_os = "linux", target_os = "unix"))]
fn terminate(pid: Pid) -> RtckResult<()> {
    use nix::sys::signal::{kill, Signal};
    // the hypervisor occupies the pid by opening fd to it (procfs).
    // so kill -9 to this pid is safe.
    kill(pid, Signal::SIGTERM).map_err(|_| RtckError::Machine("fail to terminate".to_string()))
}

/// Terminate the hypervisor by sending SIGKILL
/// Note: this command will kill firecracker itself.
#[cfg(any(target_os = "linux", target_os = "unix"))]
fn kill(pid: Pid) -> RtckResult<()> {
    // the hypervisor occupies the pid by opening fd to it (procfs).
    // so kill -9 to this pid is safe.
    use nix::sys::signal::{kill, Signal};
    kill(pid, Signal::SIGKILL)
        // kill -9 should not trigger this error since SIGKILL is not blockable
        .map_err(|_| RtckError::Machine("fail to terminate".to_string()))
}

// 定义一个操作的回滚函数类型
// pub type RollbackFn = Box<dyn FnOnce() + Send + 'static>;

pub enum Rollback {
    Jailing {
        instance_dir: PathBuf,
    },
    StopProcess {
        pid: u32,
    },
    RemoveSocket {
        path: PathBuf,
    },
    RemoveFsLock {
        path: PathBuf,
    },
    Umount {
        mount_point: PathBuf,
    },
    Chown {
        path: PathBuf,
        original_uid: u32,
        original_gid: u32,
    },
}

impl Rollback {
    fn rollback(self) {
        match self {
            Rollback::Jailing { instance_dir } => {
                // remove the instance directory
                log::info!("Removing instance dir {:?}", instance_dir);
                let _ = std::fs::remove_dir_all(instance_dir);
            }
            Rollback::StopProcess { pid } => {
                log::info!("Terminating process {pid}");
                use nix::{
                    sys::wait::{waitpid, WaitStatus},
                    unistd::Pid,
                };

                let pid = Pid::from_raw(pid as i32);
                if terminate(pid).is_err() {
                    let _ = kill(pid);
                }

                loop {
                    match waitpid(pid, None) {
                        Ok(WaitStatus::Exited(_, status)) => {
                            log::info!("Process {} exited with status {}", pid, status);
                            break;
                        }
                        Ok(WaitStatus::Signaled(_, signal, _)) => {
                            log::warn!("Process {} was killed by signal {}", pid, signal);
                            break;
                        }
                        Ok(_) => {
                            // other WaitStatus，e.g. Stopped、Continued
                            if terminate(pid).is_err() {
                                let _ = kill(pid);
                            }
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                        Err(nix::errno::Errno::ECHILD) => {
                            log::error!("No such process {}", pid);
                            break;
                        }
                        Err(err) => {
                            log::error!("Error occurred: {}", err);
                            break;
                        }
                    }
                }
            }
            Rollback::RemoveSocket { path } => {
                // removal failure is not a big deal so ignore possible error
                log::info!("Removing socket {:?}", path);
                let _ = std::fs::remove_file(path);
            }
            Rollback::RemoveFsLock { path } => {
                // removal failure is not a big deal so ignore possible error
                log::info!("Removing lock {:?}", path);
                let _ = std::fs::remove_file(path);
            }
            Rollback::Umount { mount_point } => {
                log::info!("Umount {mount_point:?}");
                use nix::mount::{umount2, MntFlags};
                let _ = umount2(&mount_point, MntFlags::MNT_FORCE).map_err(|e| {
                    RtckError::Hypervisor(format!("fail to umount the kernel dir, errno = {}", e))
                });
            }
            Rollback::Chown {
                path,
                original_uid,
                original_gid,
            } => {
                log::info!(
                    "Change onwership of {:?} back to ({}:{})",
                    path,
                    original_uid,
                    original_gid
                );
                use nix::unistd::{Gid, Uid};
                let _ = nix::unistd::chown(
                    &path,
                    Some(Uid::from_raw(original_uid)),
                    Some(Gid::from_raw(original_gid)),
                );
            }
        }
    }
}

// RollbackStack 用于管理回滚函数的栈
pub struct RollbackStack {
    pub stack: Vec<Rollback>,
}

impl RollbackStack {
    pub fn new() -> Self {
        RollbackStack { stack: Vec::new() }
    }

    // 添加一个回滚函数到栈中
    pub fn push(&mut self, rollback: Rollback) {
        self.stack.push(rollback);
    }

    pub fn insert_1(&mut self, rollback: Rollback) {
        self.stack.insert(self.stack.len() - 1, rollback);
    }

    // 执行所有的回滚函数，按LIFO顺序
    pub fn rollback_all(&mut self) {
        while let Some(op) = self.stack.pop() {
            op.rollback();
        }
    }
}

// 自动在作用域结束时调用回滚函数
impl Drop for RollbackStack {
    fn drop(&mut self) {
        self.rollback_all();
    }
}

// // 示例操作函数，返回Result并携带其回滚函数
// fn perform_operation(i: usize) -> Result<Rollback, Box<dyn Error>> {
//     println!("Performing operation {}", i);
//     if i == 8 {
//         return Err(Box::new(MyError(format!("Error in operation {}", i))));
//     }
//     // Ok(Box::new(move || println!("Rollback operation {}", i)))
//     if i % 2 == 0 {
//         Ok(Rollback::StopProcess { pid: i as u32 })
//     } else {
//         Ok(Rollback::Umount {
//             source: format!("source {i}"),
//             mount_point: format!("mount_point {i}"),
//         })
//     }
// }

// fn main() -> Result<(), Box<dyn Error>> {
//     let mut rollback_stack = RollbackStack::new();

//     for i in 0..10 {
//         match perform_operation(i) {
//             Ok(rollback_fn) => {
//                 rollback_stack.push(rollback_fn);
//             }
//             Err(e) => {
//                 println!("Error occurred: {}", e);
//                 // 发生错误，回滚所有操作
//                 return Err(e);
//             }
//         }
//     }

//     let size = size_of_val(&rollback_stack.stack[..]);
//     println!("size of roll_back_stack = {size}");

//     // 如果所有操作都成功，清空回滚栈以避免回滚
//     std::mem::forget(rollback_stack);

//     Ok(())
// }
