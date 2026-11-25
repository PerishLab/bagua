use super::Process;
use crate::{Exit, Failure, GRACE, Stage};
use std::{
    io::{self, Write as _},
    time::{Duration, Instant},
};
use windows_sys::Win32::System::Threading::TerminateProcess;

impl Process {
    pub fn retire(&mut self) -> Result<(), Failure> {
        self.collect()?;
        self.writer = None;
        self.reader = None;
        self.console = None;
        if self.exit.is_some() || self.deadline.is_some() {
            return Ok(());
        }
        self.deadline = Some(Instant::now() + GRACE);
        Ok(())
    }

    pub fn force(&mut self) -> Result<(), Failure> {
        self.collect()?;
        let Some(child) = &self.child else {
            return Ok(());
        };
        if unsafe { TerminateProcess(child.raw(), 1) } == 0 {
            let error = io::Error::last_os_error();
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
            let _ = writeln!(io::stderr(), "bagua PTY cleanup failed: {error}");
        }
    }
}
