use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub(super) fn location(path: &Path, managed: bool, create: bool) -> io::Result<PathBuf> {
    if !managed {
        return Ok(path.into());
    }
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("default socket has no parent"))?;
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    if create {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)?;
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    if create && !parent.exists() {
        fs::create_dir_all(parent)?;
        bagua::ipc::secure(parent)?;
    }
    let parent = crate::config::strip(fs::canonicalize(parent)?);
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::other("default socket has no name"))?;
    Ok(parent.join(name))
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) fn image(file: &Path) -> io::Result<protocol::wire::Image> {
    use std::os::unix::fs::MetadataExt;
    let running = std::fs::metadata(file)?;
    Ok(protocol::wire::Image {
        device: running.dev(),
        inode: running.ino(),
    })
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub(crate) fn image(file: &Path) -> io::Result<protocol::wire::Image> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle},
    };
    let handle = std::fs::File::open(file)?;
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe { GetFileInformationByHandle(handle.as_raw_handle() as HANDLE, &mut info) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(protocol::wire::Image {
        device: info.dwVolumeSerialNumber as u64,
        inode: ((info.nFileIndexHigh as u64) << 32) | (info.nFileIndexLow as u64),
    })
}
