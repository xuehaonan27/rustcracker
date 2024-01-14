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
    firecracker_binary_path: PathBuf,
    socket_connection_pool: SocketConnectionPool,
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

pub fn launch(path: &PathBuf, socket_path: &PathBuf) -> Result<Child> {
    let child = Command::new("sudo")
        .arg(path)
        .arg("--api-sock")
        .arg(socket_path)
        // .stdout(Stdio::null())
        .spawn()?;
    Ok(child)
}

pub fn wait_socket(socket_path: &PathBuf) {
    while let Err(_) = std::fs::metadata(socket_path) {}
}

impl Agent {}
