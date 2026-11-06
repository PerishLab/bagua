mod close;
mod io;
mod spawn;

use crate::{Exit, Failure, Launch, Stage, Status};
use std::{
    os::{fd::OwnedFd, unix::process::ExitStatusExt},
    process::Child,
    time::Instant,
};

pub struct Process {
    master: Option<OwnedFd>,
    child: Option<Child>,
    id: u32,
    exit: Option<Exit>,
    deadline: Option<Instant>,
    forced: bool,
}

impl Process {
    pub fn spawn(launch: Launch) -> Result<Self, Failure> {
        spawn::start(launch)
    }
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn status(&self) -> Status {
        Status {
            exit: self.exit,
            io: self.master.is_some(),
            closing: self.deadline.is_some() && self.exit.is_none(),
        }
    }

    pub fn observe(&mut self) -> Result<Option<Exit>, Failure> {
        self.collect()?;
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
            && !self.forced
        {
            self.force()?;
            self.collect()?;
        }
        Ok(self.exit)
    }

    fn collect(&mut self) -> Result<(), Failure> {
        let Some(child) = &mut self.child else {
            return Ok(());
        };
        if let Some(status) = child
            .try_wait()
            .map_err(|error| Failure::io(Stage::Wait, error))?
        {
            self.exit = Some(Exit {
                code: status.code(),
                signal: status.signal(),
            });
            self.child = None;
            self.deadline = None;
        }
        Ok(())
    }

    fn master(&self) -> Result<&OwnedFd, Failure> {
        self.master.as_ref().ok_or(Failure::Closed)
    }
}
