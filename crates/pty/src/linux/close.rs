use super::Process;
use crate::{Exit, Failure, GRACE, Stage};
use rustix::process::{Pid, Signal, kill_process};
use std::{
    io::Write as _,
    time::{Duration, Instant},
};

impl Process {
    pub fn retire(&mut self) -> Result<(), Failure> {
        self.collect()?;
        self.master = None;
        if self.exit.is_some() || self.deadline.is_some() {
            return Ok(());
        }
        self.deadline = Some(Instant::now() + GRACE);
        if let Some(child) = &self.child
            && let Err(error) = kill_process(Pid::from_child(child), Signal::HUP)
        {
            self.collect()?;
            if self.exit.is_none() {
                return Err(Failure::io(Stage::Close, error));
            }
        }
        Ok(())
    }

    pub fn force(&mut self) -> Result<(), Failure> {
        self.collect()?;
        let result = match &mut self.child {
            Some(child) => child.kill(),
            None => return Ok(()),
        };
        if let Err(error) = result {
            self.collect()?;
            if self.exit.is_none() {
                return Err(Failure::io(Stage::Close, error));
            }
        }
        self.forced = true;
        Ok(())
    }

    pub fn close(&mut self) -> Result<Exit, Failure> {
        self.retire()?;
        loop {
            if let Some(exit) = self.observe()? {
                return Ok(exit);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            let _ = writeln!(std::io::stderr(), "bagua PTY cleanup failed: {error}");
        }
    }
}
