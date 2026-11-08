use std::{
    fs, io,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
};

pub(super) fn location(path: &Path, managed: bool, create: bool) -> io::Result<PathBuf> {
    if !managed {
        return Ok(path.into());
    }
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("default socket has no parent"))?;
    if create {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)?;
    }
    let parent = fs::canonicalize(parent)?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::other("default socket has no name"))?;
    Ok(parent.join(name))
}
