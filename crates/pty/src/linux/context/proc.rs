use rustix::fs::{Mode, OFlags, open, openat};
use std::{
    fs::File,
    io::{self, Read},
    os::{fd::OwnedFd, unix::fs::MetadataExt},
};

pub(super) struct Proc {
    pub directory: OwnedFd,
}

#[derive(PartialEq, Eq)]
pub(super) struct Record {
    pub pid: u32,
    pub group: u32,
    pub session: u32,
    pub started: u64,
    pub command: String,
}

impl Proc {
    pub fn open(pid: u32) -> io::Result<Self> {
        let directory = open(
            format!("/proc/{pid}"),
            OFlags::PATH | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )?;
        if rustix::fs::fstat(&directory)?.st_uid != rustix::process::geteuid().as_raw() {
            return Err(io::Error::other(
                "foreground process belongs to another user",
            ));
        }
        Ok(Self { directory })
    }

    pub fn record(&self) -> io::Result<Record> {
        let descriptor = openat(
            &self.directory,
            "stat",
            OFlags::RDONLY | OFlags::CLOEXEC,
            Mode::empty(),
        )?;
        let mut text = String::new();
        File::from(descriptor)
            .take(8193)
            .read_to_string(&mut text)?;
        if text.len() > 8192 {
            return Err(io::Error::other(
                "foreground process record exceeds its bound",
            ));
        }
        parse(&text)
    }

    pub fn cwd(&self) -> io::Result<std::path::PathBuf> {
        use std::os::unix::ffi::OsStringExt;
        rustix::fs::accessat(
            &self.directory,
            "cwd",
            rustix::fs::Access::EXEC_OK,
            rustix::fs::AtFlags::EACCESS,
        )?;
        let mut buffer = [0; 4097];
        let count = rustix::fs::readlinkat_raw(&self.directory, "cwd", &mut buffer[..])?;
        if count == 0 || count > 4096 {
            return Err(io::Error::other("foreground cwd exceeds its bound"));
        }
        let path = std::path::PathBuf::from(std::ffi::OsString::from_vec(buffer[..count].to_vec()));
        if !path.is_absolute() || path.to_str().is_none() {
            return Err(io::Error::other("foreground cwd must be absolute UTF-8"));
        }
        let directory = openat(
            &self.directory,
            "cwd",
            OFlags::PATH | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )?;
        let owned = File::from(directory).metadata()?;
        let observed = std::fs::metadata(&path)?;
        if owned.dev() != observed.dev() || owned.ino() != observed.ino() {
            return Err(io::Error::other("foreground cwd moved or was replaced"));
        }
        Ok(path)
    }
}

fn parse(text: &str) -> io::Result<Record> {
    let (prefix, suffix) = text
        .rsplit_once(')')
        .ok_or_else(|| io::Error::other("invalid process record"))?;
    let (pid, command) = prefix
        .split_once('(')
        .ok_or_else(|| io::Error::other("invalid process command"))?;
    let fields: Vec<_> = suffix.split_whitespace().collect();
    if matches!(fields.first().copied(), Some("Z" | "X")) {
        return Err(io::Error::other("foreground process has exited"));
    }
    Ok(Record {
        pid: pid
            .trim()
            .parse()
            .map_err(|_| io::Error::other("invalid process identity"))?,
        group: number(&fields, 2)?,
        session: number(&fields, 3)?,
        started: fields
            .get(19)
            .ok_or_else(|| io::Error::other("missing process lifetime"))?
            .parse()
            .map_err(|_| io::Error::other("invalid process lifetime"))?,
        command: command.into(),
    })
}

fn number(fields: &[&str], index: usize) -> io::Result<u32> {
    fields
        .get(index)
        .ok_or_else(|| io::Error::other("missing process membership"))?
        .parse()
        .map_err(|_| io::Error::other("invalid process membership"))
}
