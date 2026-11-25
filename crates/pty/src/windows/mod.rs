mod close;
mod env;
mod handle;
mod pipe;
mod reader;
mod spawn;
mod writer;

use crate::{Exit, Failure, Interest, Launch, Ready, Size, Stage, Status, Transfer};
use handle::{Console, Handle};
use reader::Reader;
use std::{
    io,
    sync::atomic::{AtomicU32, Ordering},
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{HANDLE, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::{
        Console::ResizePseudoConsole,
        Threading::{GetExitCodeProcess, WaitForMultipleObjects, WaitForSingleObject},
    },
};
use writer::Writer;

pub struct Process {
    console: Option<Console>,
    reader: Option<Reader>,
    writer: Option<Writer>,
    child: Option<Handle>,
    id: u32,
    size: AtomicU32,
    exit: Option<Exit>,
    deadline: Option<Instant>,
    forced: bool,
}

impl Process {
    pub fn context(&self) -> Result<crate::Context, Failure> {
        Err(Failure::Unsupported)
    }

    pub fn spawn(launch: Launch) -> Result<Self, Failure> {
        spawn::start(launch)
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn status(&self) -> Status {
        Status {
            exit: self.exit,
            io: self.console.is_some(),
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

    pub fn size(&self) -> Result<Size, Failure> {
        if self.console.is_none() {
            return Err(Failure::Closed);
        }
        let packed = self.size.load(Ordering::Relaxed);
        let columns = (packed >> 16) as u16;
        let rows = (packed & 0xffff) as u16;
        Size::new(columns, rows).ok_or(Failure::Input("kernel reported zero PTY size"))
    }

    pub fn resize(&self, size: Size) -> Result<(), Failure> {
        let console = self.console.as_ref().ok_or(Failure::Closed)?;
        let status = unsafe { ResizePseudoConsole(console.raw(), spawn::coord(size)) };
        if status != 0 {
            return Err(Failure::io(
                Stage::Resize,
                io::Error::from_raw_os_error(status),
            ));
        }
        self.size.store(spawn::pack(size), Ordering::Relaxed);
        Ok(())
    }

    pub fn read(&mut self, buffer: &mut [u8]) -> Result<Transfer, Failure> {
        if buffer.is_empty() {
            return Err(Failure::Input("read buffer must not be empty"));
        }
        let reader = self.reader.as_mut().ok_or(Failure::Closed)?;
        let transfer = reader.read(buffer)?;
        if transfer == Transfer::Pending && self.exit.is_some() && self.console.is_some() {
            self.console = None;
            return self.reader.as_mut().ok_or(Failure::Closed)?.read(buffer);
        }
        Ok(transfer)
    }

    pub fn write(&mut self, buffer: &[u8]) -> Result<Transfer, Failure> {
        let writer = self.writer.as_mut().ok_or(Failure::Closed)?;
        writer.write(buffer)
    }

    pub fn poll(&self, interest: Interest, timeout: Duration) -> Result<Ready, Failure> {
        if timeout > Duration::from_secs(1) {
            return Err(Failure::Input("poll timeout exceeds one second"));
        }
        let reader = self.reader.as_ref().ok_or(Failure::Closed)?;
        let writer = self.writer.as_ref().ok_or(Failure::Closed)?;
        let immediate = state(reader, writer, interest);
        if immediate.read || immediate.write || immediate.hangup {
            return Ok(immediate);
        }
        let mut handles: [HANDLE; 2] = [std::ptr::null_mut(); 2];
        let mut count = 0usize;
        if matches!(interest, Interest::Read | Interest::Both) {
            handles[count] = reader.event();
            count += 1;
        }
        if matches!(interest, Interest::Write | Interest::Both) {
            handles[count] = writer.event();
            count += 1;
        }
        let millis = timeout.as_millis().min(1000) as u32;
        let waited = unsafe { WaitForMultipleObjects(count as u32, handles.as_ptr(), 0, millis) };
        if waited == WAIT_FAILED {
            return Err(Failure::io(Stage::Poll, io::Error::last_os_error()));
        }
        if waited == WAIT_TIMEOUT {
            return Ok(Ready::default());
        }
        Ok(state(reader, writer, interest))
    }

    fn collect(&mut self) -> Result<(), Failure> {
        let Some(child) = &self.child else {
            return Ok(());
        };
        match unsafe { WaitForSingleObject(child.raw(), 0) } {
            WAIT_OBJECT_0 => {
                let mut code = 0u32;
                if unsafe { GetExitCodeProcess(child.raw(), &mut code) } == 0 {
                    return Err(Failure::io(Stage::Wait, io::Error::last_os_error()));
                }
                self.exit = Some(Exit {
                    code: Some(code as i32),
                    signal: None,
                });
                self.child = None;
                self.deadline = None;
                Ok(())
            }
            WAIT_TIMEOUT => Ok(()),
            _ => Err(Failure::io(Stage::Wait, io::Error::last_os_error())),
        }
    }
}

fn state(reader: &Reader, writer: &Writer, interest: Interest) -> Ready {
    let wantread = matches!(interest, Interest::Read | Interest::Both);
    let wantwrite = matches!(interest, Interest::Write | Interest::Both);
    Ready {
        read: wantread && reader.ready(),
        write: wantwrite && writer.ready(),
        hangup: reader.eof(),
    }
}
