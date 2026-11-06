use std::{
    fs::{self, Permissions},
    io,
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
};

pub(super) struct Socket {
    pub listener: UnixListener,
    path: PathBuf,
    device: u64,
    inode: u64,
}

pub(super) fn namespace(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::Error::other("socket path must be absolute"));
    }
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("socket has no parent"))?;
    if fs::canonicalize(parent)? != parent {
        return Err(io::Error::other("socket parent must be canonical"));
    }
    let metadata = fs::symlink_metadata(parent)?;
    if !metadata.is_dir()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o077 != 0
    {
        return Err(io::Error::other(
            "socket parent must be a private directory owned by this user",
        ));
    }
    Ok(())
}

pub(super) fn peer(stream: &UnixStream) -> io::Result<()> {
    let credentials = rustix::net::sockopt::socket_peercred(stream)?;
    if credentials.uid != rustix::process::geteuid() {
        return Err(io::Error::other("socket peer belongs to another user"));
    }
    Ok(())
}

impl Socket {
    pub fn bind(path: &Path) -> io::Result<Self> {
        namespace(path)?;
        let listener = UnixListener::bind(path)?;
        let metadata = fs::symlink_metadata(path)?;
        let socket = Self {
            listener,
            path: path.into(),
            device: metadata.dev(),
            inode: metadata.ino(),
        };
        fs::set_permissions(path, Permissions::from_mode(0o600))?;
        socket.listener.set_nonblocking(true)?;
        Ok(socket)
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        let result = (|| {
            let metadata = fs::symlink_metadata(&self.path)?;
            if metadata.dev() == self.device && metadata.ino() == self.inode {
                fs::remove_file(&self.path)?;
            }
            Ok::<(), io::Error>(())
        })();
        if let Err(error) = result
            && error.kind() != io::ErrorKind::NotFound
        {
            eprintln!("bagua socket cleanup failed: {error}");
        }
    }
}
