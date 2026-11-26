mod proc;
mod tree;

use super::Process;
use crate::Context;
use std::io;

pub(super) fn read(process: &Process) -> io::Result<Context> {
    if process.exit.is_some() || process.deadline.is_some() {
        return Err(io::Error::other("pane process is exited or retiring"));
    }
    let child = process
        .child
        .as_ref()
        .ok_or_else(|| io::Error::other("pane process handle is closed"))?;
    proc::alive(child.raw())?;
    let root = proc::Target::open(process.id)?;
    let base = root.stamp()?;
    let target = tree::select(process.id, base.started)?;
    let before = target.stamp()?;
    let command = target.command()?;
    let cwd = target.cwd()?;
    if target.stamp()? != before || root.stamp()? != base || proc::alive(child.raw()).is_err() {
        return Err(io::Error::other(
            "foreground process changed during observation",
        ));
    }
    Ok(Context {
        pid: before.pid,
        session: process.id,
        started: before.started,
        command,
        cwd,
    })
}
