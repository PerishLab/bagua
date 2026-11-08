mod proc;

use super::Process;
use crate::Context;
use std::io;

pub(super) fn read(process: &Process) -> io::Result<Context> {
    if process.exit.is_some() || process.deadline.is_some() {
        return Err(io::Error::other("pane process is exited or retiring"));
    }
    let master = process.master().map_err(io::Error::other)?;
    let foreground = rustix::termios::tcgetpgrp(master)?.as_raw_pid() as u32;
    if rustix::termios::tcgetsid(master)?.as_raw_pid() as u32 != process.id {
        return Err(io::Error::other("PTY session changed"));
    }
    let owner = proc::Proc::open(process.id)?.record()?;
    if owner.pid != process.id || owner.session != process.id {
        return Err(io::Error::other("pane process identity changed"));
    }
    let directory = proc::Proc::open(foreground)?;
    let before = directory.record()?;
    if before.pid != foreground || before.group != foreground || before.session != process.id {
        return Err(io::Error::other(
            "foreground process is outside the owned session",
        ));
    }
    let cwd = directory.cwd()?;
    if directory.record()? != before
        || rustix::termios::tcgetpgrp(master)?.as_raw_pid() as u32 != foreground
    {
        return Err(io::Error::other(
            "foreground process changed during observation",
        ));
    }
    Ok(Context {
        pid: foreground,
        session: process.id,
        started: before.started,
        command: before.command,
        cwd,
    })
}
