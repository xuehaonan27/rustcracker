use crate::{RtckError, RtckResult};
use log::*;
use nix::unistd::Pid;
use std::path::PathBuf;

/// Terminate the hypervisor by sending SIGTERM
/// Note: this command will kill firecracker itself
#[cfg(any(target_os = "linux", target_os = "unix"))]
fn terminate(pid: Pid) -> RtckResult<()> {
    use nix::sys::signal::{kill, Signal};
    // the hypervisor occupies the pid by opening fd to it (procfs).
    // so kill -9 to this pid is safe.
    kill(pid, Signal::SIGTERM).map_err(|e| {
        let msg = format!("Fail to terminate pid {pid}: {e}");
        error!("{msg}");
        RtckError::Hypervisor(msg)
    })
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
        .map_err(|e| {
            let msg = format!("Fail to kill pid {pid}: {e}");
            error!("{msg}");
            RtckError::Hypervisor(msg)
        })
}

pub enum Rollback {
    Jailing {
        clear: bool,
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
            Rollback::Jailing {
                clear,
                instance_dir,
            } => {
                // remove the instance directory
                if clear {
                    info!("Removing instance dir {:?}", instance_dir);
                    let _ = std::fs::remove_dir_all(instance_dir);
                }
            }
            Rollback::StopProcess { pid } => {
                info!("Terminating process {pid}");
                use nix::{
                    sys::wait::{waitpid, WaitPidFlag, WaitStatus},
                    unistd::Pid,
                };
                let pid = Pid::from_raw(pid as i32);

                // first check the status of process.
                // if user has give up this microVM voluntarily
                loop {
                    match waitpid(pid, Some(WaitPidFlag::WNOHANG)) {
                        Ok(WaitStatus::Exited(_, exit_status)) => {
                            warn!("Process {pid} has exited {exit_status}");
                            return;
                        }
                        Ok(WaitStatus::Signaled(_, signal, core_dumped)) => {
                            warn!(
                            "Process {pid} has been signaled {signal}, core_dumped: {core_dumped}"
                        );
                            return;
                        }
                        Ok(WaitStatus::StillAlive)
                        | Ok(WaitStatus::PtraceEvent(_, _, _))
                        | Ok(WaitStatus::PtraceSyscall(_))
                        | Ok(WaitStatus::Continued(_)) => break, // terminate it!
                        Err(nix::errno::Errno::ECHILD) => {
                            error!("No such process {}", pid);
                            return;
                        }
                        Err(nix::errno::Errno::EINTR) => {
                            warn!("Checking status interrupted, trying again...");
                            continue;
                        }
                        Err(nix::errno::Errno::EINVAL) => {
                            error!(
                                "Checking status invalid arguments, send a terminate signal anyway"
                            );
                            break;
                        }
                        _ => {
                            error!("Fatal error! unreachable");
                            return;
                        }
                    }
                }

                if self::terminate(pid).is_err() {
                    let _ = self::kill(pid);
                }

                loop {
                    match waitpid(pid, None) {
                        Ok(WaitStatus::Exited(_, status)) => {
                            info!("Process {} exited with status {}", pid, status);
                            break;
                        }
                        Ok(WaitStatus::Signaled(_, signal, _)) => {
                            warn!("Process {} was killed by signal {}", pid, signal);
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
                            error!("No such process {}", pid);
                            break;
                        }
                        Err(err) => {
                            error!("Error occurred: {}", err);
                            break;
                        }
                    }
                }
            }
            Rollback::RemoveSocket { path } => {
                // removal failure is not a big deal so ignore possible error
                info!("Removing socket {:?}", path);
                let _ = std::fs::remove_file(path);
            }
            Rollback::RemoveFsLock { path } => {
                // removal failure is not a big deal so ignore possible error
                info!("Removing lock {:?}", path);
                let _ = std::fs::remove_file(path);
            }
            Rollback::Umount { mount_point } => {
                info!("Umount {mount_point:?}");
                use nix::mount::{umount2, MntFlags};
                let _ = umount2(&mount_point, MntFlags::MNT_FORCE).map_err(|e| {
                    RtckError::Hypervisor(format!("Fail to umount the kernel dir, errno = {}", e))
                });
            }
            Rollback::Chown {
                path,
                original_uid,
                original_gid,
            } => {
                info!(
                    "Change onwership of {:?} back to ({}:{})",
                    path, original_uid, original_gid
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

/// Stack that manages rollbacks.
pub struct RollbackStack {
    pub stack: Vec<Rollback>,
}

impl RollbackStack {
    pub fn new() -> Self {
        RollbackStack { stack: Vec::new() }
    }

    pub fn push(&mut self, rollback: Rollback) {
        self.stack.push(rollback);
    }

    pub fn insert_1(&mut self, rollback: Rollback) {
        self.stack.insert(self.stack.len() - 1, rollback);
    }

    pub fn rollback_all(&mut self) {
        while let Some(op) = self.stack.pop() {
            op.rollback();
        }
    }
}

impl Drop for RollbackStack {
    fn drop(&mut self) {
        self.rollback_all();
    }
}
