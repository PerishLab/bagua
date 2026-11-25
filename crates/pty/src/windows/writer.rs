use super::{
    handle::Handle,
    pipe::{Slot, cancel, code},
};
use crate::{CHUNK, Failure, Stage, Transfer};
use std::io;
use windows_sys::Win32::{
    Foundation::{
        ERROR_BROKEN_PIPE, ERROR_IO_INCOMPLETE, ERROR_IO_PENDING, ERROR_PIPE_NOT_CONNECTED, HANDLE,
        WAIT_OBJECT_0,
    },
    Storage::FileSystem::WriteFile,
    System::{IO::GetOverlappedResult, Threading::WaitForSingleObject},
};

pub(super) struct Writer {
    handle: Handle,
    slot: Box<Slot>,
    pending: bool,
    closed: bool,
}

impl Writer {
    pub fn new(handle: Handle) -> io::Result<Self> {
        Ok(Self {
            handle,
            slot: Slot::new()?,
            pending: false,
            closed: false,
        })
    }

    pub fn write(&mut self, buffer: &[u8]) -> Result<Transfer, Failure> {
        self.harvest()
            .map_err(|error| Failure::io(Stage::Write, error))?;
        if self.closed {
            return Ok(Transfer::Eof);
        }
        if self.pending {
            return Ok(Transfer::Pending);
        }
        let length = buffer.len().min(CHUNK);
        if length == 0 {
            return Ok(Transfer::Bytes(0));
        }
        self.slot
            .reset()
            .map_err(|error| Failure::io(Stage::Write, error))?;
        self.slot.buffer[..length].copy_from_slice(&buffer[..length]);
        let mut written = 0u32;
        let ok = unsafe {
            WriteFile(
                self.handle.raw(),
                self.slot.buffer.as_ptr(),
                length as u32,
                &mut written,
                &mut self.slot.overlapped,
            )
        };
        if ok != 0 {
            return Ok(Transfer::Bytes(written as usize));
        }
        match code() {
            ERROR_IO_PENDING => {
                self.pending = true;
                Ok(Transfer::Bytes(length))
            }
            ERROR_BROKEN_PIPE | ERROR_PIPE_NOT_CONNECTED => {
                self.closed = true;
                Ok(Transfer::Eof)
            }
            other => Err(Failure::io(
                Stage::Write,
                io::Error::from_raw_os_error(other as i32),
            )),
        }
    }

    pub fn ready(&self) -> bool {
        !self.pending || unsafe { WaitForSingleObject(self.slot.event.raw(), 0) } == WAIT_OBJECT_0
    }

    pub fn event(&self) -> HANDLE {
        self.slot.event.raw()
    }

    fn harvest(&mut self) -> io::Result<()> {
        if !self.pending {
            return Ok(());
        }
        let mut transferred = 0u32;
        let ok = unsafe {
            GetOverlappedResult(
                self.handle.raw(),
                &self.slot.overlapped,
                &mut transferred,
                0,
            )
        };
        if ok != 0 {
            self.pending = false;
            return Ok(());
        }
        match code() {
            ERROR_IO_INCOMPLETE => Ok(()),
            ERROR_BROKEN_PIPE | ERROR_PIPE_NOT_CONNECTED => {
                self.pending = false;
                self.closed = true;
                Ok(())
            }
            other => {
                self.pending = false;
                Err(io::Error::from_raw_os_error(other as i32))
            }
        }
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        if self.pending {
            cancel(self.handle.raw(), &self.slot.overlapped);
            self.pending = false;
        }
    }
}
