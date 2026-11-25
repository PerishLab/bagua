use super::handle::{Handle, code};
use std::{io, mem::zeroed, ptr};
use windows_sys::Win32::{
    Foundation::{
        ERROR_BROKEN_PIPE, ERROR_HANDLE_EOF, ERROR_IO_INCOMPLETE, ERROR_IO_PENDING,
        ERROR_PIPE_NOT_CONNECTED, HANDLE,
    },
    Storage::FileSystem::{ReadFile, WriteFile},
    System::{
        IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED},
        Threading::{CreateEventW, ResetEvent},
    },
};

const BUFFER: usize = 65_536;

struct Slot {
    overlapped: OVERLAPPED,
    bytes: [u8; BUFFER],
    event: Handle,
}

unsafe impl Send for Slot {}
unsafe impl Sync for Slot {}

impl Slot {
    fn new() -> io::Result<Box<Self>> {
        let event = Handle::new(unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) })?;
        let mut overlapped: OVERLAPPED = unsafe { zeroed() };
        overlapped.hEvent = event.raw();
        Ok(Box::new(Self {
            overlapped,
            bytes: [0; BUFFER],
            event,
        }))
    }

    fn reset(&mut self) -> io::Result<()> {
        if unsafe { ResetEvent(self.event.raw()) } == 0 {
            return Err(io::Error::last_os_error());
        }
        self.overlapped = unsafe { zeroed() };
        self.overlapped.hEvent = self.event.raw();
        Ok(())
    }
}

pub(super) struct Duplex {
    handle: Handle,
    incoming: Box<Slot>,
    outgoing: Box<Slot>,
    reading: bool,
    writing: bool,
    eof: bool,
    broken: bool,
    ready: usize,
    offset: usize,
}

impl Duplex {
    pub fn new(handle: Handle) -> io::Result<Self> {
        Ok(Self {
            handle,
            incoming: Slot::new()?,
            outgoing: Slot::new()?,
            reading: false,
            writing: false,
            eof: false,
            broken: false,
            ready: 0,
            offset: 0,
        })
    }

    pub fn read(&mut self, target: &mut [u8]) -> io::Result<usize> {
        self.harvest()?;
        if self.offset < self.ready {
            return Ok(self.take(target));
        }
        if self.eof {
            return Ok(0);
        }
        if !self.reading {
            self.arm()?;
            if self.offset < self.ready {
                return Ok(self.take(target));
            }
            if self.eof {
                return Ok(0);
            }
        }
        Err(io::ErrorKind::WouldBlock.into())
    }

    pub fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.flush()?;
        if self.broken {
            return Ok(0);
        }
        if self.writing {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let length = data.len().min(BUFFER);
        if length == 0 {
            return Ok(0);
        }
        self.outgoing.reset()?;
        self.outgoing.bytes[..length].copy_from_slice(&data[..length]);
        let mut written = 0u32;
        let ok = unsafe {
            WriteFile(
                self.handle.raw(),
                self.outgoing.bytes.as_ptr(),
                length as u32,
                &mut written,
                &mut self.outgoing.overlapped,
            )
        };
        if ok != 0 {
            return Ok(written as usize);
        }
        match code() {
            ERROR_IO_PENDING => {
                self.writing = true;
                Ok(length)
            }
            ERROR_BROKEN_PIPE | ERROR_PIPE_NOT_CONNECTED => {
                self.broken = true;
                Ok(0)
            }
            other => Err(io::Error::from_raw_os_error(other as i32)),
        }
    }

    fn take(&mut self, target: &mut [u8]) -> usize {
        let count = target.len().min(self.ready - self.offset);
        target[..count].copy_from_slice(&self.incoming.bytes[self.offset..self.offset + count]);
        self.offset += count;
        if self.offset == self.ready {
            self.offset = 0;
            self.ready = 0;
        }
        count
    }

    fn arm(&mut self) -> io::Result<()> {
        self.incoming.reset()?;
        let mut count = 0u32;
        let ok = unsafe {
            ReadFile(
                self.handle.raw(),
                self.incoming.bytes.as_mut_ptr(),
                BUFFER as u32,
                &mut count,
                &mut self.incoming.overlapped,
            )
        };
        if ok != 0 {
            if count == 0 {
                self.eof = true;
            } else {
                self.ready = count as usize;
                self.offset = 0;
            }
            return Ok(());
        }
        match code() {
            ERROR_IO_PENDING => {
                self.reading = true;
                Ok(())
            }
            ERROR_BROKEN_PIPE | ERROR_PIPE_NOT_CONNECTED | ERROR_HANDLE_EOF => {
                self.eof = true;
                Ok(())
            }
            other => Err(io::Error::from_raw_os_error(other as i32)),
        }
    }

    fn harvest(&mut self) -> io::Result<()> {
        if !self.reading {
            return Ok(());
        }
        let mut count = 0u32;
        let ok = unsafe {
            GetOverlappedResult(self.handle.raw(), &self.incoming.overlapped, &mut count, 0)
        };
        if ok != 0 {
            self.reading = false;
            if count == 0 {
                self.eof = true;
            } else {
                self.ready = count as usize;
                self.offset = 0;
            }
            return Ok(());
        }
        match code() {
            ERROR_IO_INCOMPLETE => Ok(()),
            ERROR_BROKEN_PIPE | ERROR_PIPE_NOT_CONNECTED | ERROR_HANDLE_EOF => {
                self.reading = false;
                self.eof = true;
                Ok(())
            }
            other => {
                self.reading = false;
                Err(io::Error::from_raw_os_error(other as i32))
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.writing {
            return Ok(());
        }
        let mut count = 0u32;
        let ok = unsafe {
            GetOverlappedResult(self.handle.raw(), &self.outgoing.overlapped, &mut count, 0)
        };
        if ok != 0 {
            self.writing = false;
            return Ok(());
        }
        match code() {
            ERROR_IO_INCOMPLETE => Ok(()),
            ERROR_BROKEN_PIPE | ERROR_PIPE_NOT_CONNECTED => {
                self.writing = false;
                self.broken = true;
                Ok(())
            }
            other => {
                self.writing = false;
                Err(io::Error::from_raw_os_error(other as i32))
            }
        }
    }
}

impl Drop for Duplex {
    fn drop(&mut self) {
        if self.reading {
            cancel(self.handle.raw(), &self.incoming.overlapped);
            self.reading = false;
        }
        if self.writing {
            cancel(self.handle.raw(), &self.outgoing.overlapped);
            self.writing = false;
        }
    }
}

fn cancel(handle: HANDLE, overlapped: &OVERLAPPED) {
    unsafe {
        CancelIoEx(handle, overlapped);
        let mut count = 0u32;
        GetOverlappedResult(handle, overlapped, &mut count, 1);
    }
}
