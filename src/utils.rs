use serde::{Deserialize, Serialize};

// use crate::{firecracker_config::FirecrackerConfig, jailer::JailerConfig, client::SingleClient, KERNEL_IMAGE_PATH, ROOTFS_PATH};
pub trait Json<'a> {
    type Item;
    // fn from_json(s: &'a String) -> serde_json::Result<Self::Item>
    // where
    //     <Self as Json<'a>>::Item: Deserialize<'a>,
    // {
    //     let b: Self::Item = serde_json::from_str(s.as_str())?;
    //     Ok(b)
    // }

    fn from_json(s: &'a str) -> serde_json::Result<Self::Item>
    where
        <Self as Json<'a>>::Item: Deserialize<'a>,
    {
        let b: Self::Item = serde_json::from_str(s)?;
        Ok(b)
    }

    fn to_json(&self) -> serde_json::Result<String>
    where
        Self: Serialize,
    {
        let s: String = serde_json::to_string(self)?;
        Ok(s)
    }

    fn into_json(self) -> serde_json::Result<String>
    where
        Self: Serialize + Sized,
    {
        let s: String = serde_json::to_string(&self)?;
        Ok(s)
    }
}

// pub fn write_firecracker_config_to_file_with_path(
//     f_config: FirecrackerConfig,
//     path: impl Into<PathBuf>,
// ) -> io::Result<()> {
//     let path: PathBuf = path.into();
//     let json = f_config.to_json()?;
//     let f = File::create(path)?;
//     let mut writer = BufWriter::new(f);
//     writer.write_all(json.as_bytes())?;
//     Ok(())
// }

// pub fn load_firecracker_config(path: impl Into<PathBuf>) -> io::Result<FirecrackerConfig> {
//     let mut f = File::open(path.into())?;
//     let mut s = String::new();
//     let _cnt = f.read_to_string(&mut s)?;
//     let firecracker_config = FirecrackerConfig::from_json(&s)?;
//     Ok(firecracker_config)
// }

// pub fn write_jailer_config_to_file_with_path(
//     config: JailerConfig,
//     path: impl Into<PathBuf>,
// ) -> io::Result<()> {
//     let path: PathBuf = path.into();
//     let json = config.to_json()?;
//     let f = File::create(path)?;
//     let mut writer = BufWriter::new(f);
//     writer.write_all(json.as_bytes())?;
//     Ok(())
// }


// pub fn prepare_file(client: &SingleClient) -> io::Result<(PathBuf, PathBuf, PathBuf)> {
//     let chroot_dir = client.get_chroot_dir();
//     let resource_dir = chroot_dir.join("resource");
//     fs::create_dir(&resource_dir)?;

//     let kernel_image_path = "vmlinux.bin";
//     let rootfs_path = "rootfs.ext4";
//     let logger_path = "firecracker.log";

//     fs::hard_link(KERNEL_IMAGE_PATH, resource_dir.join(kernel_image_path))?;
//     fs::hard_link(ROOTFS_PATH, resource_dir.join(rootfs_path))?;

//     // #[cfg(any(
//     //     target_os = "dragonfly",
//     //     target_os = "freebsd",
//     //     target_os = "macos",
//     //     target_os = "netbsd",
//     //     target_os = "openbsd"
//     // ))]
//     // std::os::unix::fs::symlink(ROOTFS_PATH, resource_dir.join(rootfs_path))?;

//     // #[cfg(any(target_os = "android", target_os = "linux"))]
//     // mount::mount(Some(ROOTFS_PATH), &resource_dir.join(rootfs_path), Some("ext4"), mount::MsFlags::MS_BIND, None::<&str>);

//     let path1 = PathBuf::from("./resource").join(kernel_image_path);
//     let path2 = PathBuf::from("./resource").join(rootfs_path);
//     let path3 = PathBuf::from("./resource").join(logger_path);
//     Ok((path1, path2, path3))
// }
