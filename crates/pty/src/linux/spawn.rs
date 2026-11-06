use super::Process;
use crate::{Failure, Launch, Size, Stage};
use rustix::{
    fd::OwnedFd,
    pty::{OpenptFlags, grantpt, ioctl_tiocgptpeer, openpt, unlockpt},
    termios::{Winsize, tcsetwinsize},
};
use std::{
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};

pub(super) fn start(launch: Launch) -> Result<Process, Failure> {
    launch.check()?;
    let (master, slave) = open(launch.size)?;
    let mut command = Command::new(&launch.program);
    command.args(&launch.args).current_dir(&launch.cwd);
    launch.environment.apply(&mut command);
    command.stdin(Stdio::from(
        slave
            .try_clone()
            .map_err(|error| Failure::io(Stage::Open, error))?,
    ));
    command.stdout(Stdio::from(
        slave
            .try_clone()
            .map_err(|error| Failure::io(Stage::Open, error))?,
    ));
    command.stderr(Stdio::from(
        slave
            .try_clone()
            .map_err(|error| Failure::io(Stage::Open, error))?,
    ));
    // SAFETY: the child closure only makes async-signal-safe setsid/ioctl calls.
    // Its captured close-on-exec slave remains owned until exec or spawn failure.
    unsafe {
        command.pre_exec(move || {
            rustix::process::setsid().map_err(std::io::Error::from)?;
            rustix::process::ioctl_tiocsctty(&slave).map_err(std::io::Error::from)?;
            Ok(())
        });
    }
    let child = command
        .spawn()
        .map_err(|error| Failure::io(Stage::Launch, error))?;
    drop(command);
    Ok(Process {
        id: child.id(),
        master: Some(master),
        child: Some(child),
        exit: None,
        deadline: None,
        forced: false,
    })
}

fn open(size: Size) -> Result<(OwnedFd, OwnedFd), Failure> {
    let flags = OpenptFlags::RDWR | OpenptFlags::NOCTTY | OpenptFlags::CLOEXEC;
    let master = openpt(flags).map_err(|error| Failure::io(Stage::Open, error))?;
    let status =
        rustix::fs::fcntl_getfl(&master).map_err(|error| Failure::io(Stage::Open, error))?;
    rustix::fs::fcntl_setfl(&master, status | rustix::fs::OFlags::NONBLOCK)
        .map_err(|error| Failure::io(Stage::Open, error))?;
    grantpt(&master).map_err(|error| Failure::io(Stage::Open, error))?;
    unlockpt(&master).map_err(|error| Failure::io(Stage::Open, error))?;
    let slave =
        ioctl_tiocgptpeer(&master, flags).map_err(|error| Failure::io(Stage::Open, error))?;
    tcsetwinsize(&master, winsize(size)).map_err(|error| Failure::io(Stage::Resize, error))?;
    Ok((master, slave))
}

pub(super) fn winsize(size: Size) -> Winsize {
    Winsize {
        ws_row: size.rows(),
        ws_col: size.columns(),
        ws_xpixel: 0,
        ws_ypixel: 0,
    }
}
