use std::{
    os::unix::fs::FileTypeExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
};

use super::connection_pool::SocketConnectionPool;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

pub struct Agent {
    socket_path: PathBuf,

    // firecracker_binary_path: PathBuf,

    /* 连接池, 用于和Firecracker通信 */
    socket_connection_pool: SocketConnectionPool,

    /* 连接池的超时请求 */
    firecracker_request_timeout: usize,

    firecracker_init_timeout: usize,
}

impl Agent {
    pub fn new(
        socket_path: impl Into<PathBuf>,
        firecracker_request_timeout: usize,
        firecracker_init_timeout: usize,
        max_conn_num: usize,
        max_pending_request_num: usize,
        worker_threads: usize,
    ) -> Self {
        let socket_path = socket_path.into();
        let socket_connection_pool = SocketConnectionPool::new(
            &socket_path,
            max_conn_num,
            max_pending_request_num,
            worker_threads,
        );
        Self {
            socket_path,
            socket_connection_pool,
            firecracker_request_timeout,
            firecracker_init_timeout,
        }
    }
}

pub fn clear(socket_path: &PathBuf) -> Result<()> {
    /* 若socket_path处已经有套接字文件则删除 */
    if let Ok(metadata) = std::fs::metadata(socket_path) {
        if metadata.file_type().is_socket() {
            std::fs::remove_file(socket_path)?;
        }
    }
    Ok(())
}

pub fn launch(
    path: &PathBuf,
    socket_path: &PathBuf,
    stdin: Option<impl Into<std::process::Stdio>>,
    stdout: Option<impl Into<std::process::Stdio>>,
    stderr: Option<impl Into<std::process::Stdio>>,
) -> Result<Child> {
    let mut child = Command::new("sudo");
    let mut child = child.arg(path).arg("--api-sock").arg(socket_path);
    if let Some(stdin) = stdin {
        child = child.stdin(stdin);
    }
    if let Some(stdout) = stdout {
        child = child.stdout(stdout);
    }
    if let Some(stderr) = stderr {
        child = child.stderr(stderr);
    }
    let child = child.spawn()?;
    Ok(child)
}

pub fn wait_socket(socket_path: &PathBuf) {
    while let Err(_) = std::fs::metadata(socket_path) {}
}
