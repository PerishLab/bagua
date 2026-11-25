use super::{
    handle::Handle,
    pipe::{Slot, cancel, code},
};
use crate::{CHUNK, Failure, Stage, Transfer};
use std::io;
use windows_sys::Win32::{
    Foundation::{
        ERROR_BROKEN_PIPE, ERROR_HANDLE_EOF, ERROR_IO_INCOMPLETE, ERROR_IO_PENDING,
        ERROR_PIPE_NOT_CONNECTED, HANDLE, WAIT_OBJECT_0,
    },
    Storage::FileSystem::ReadFile,
    System::{IO::GetOverlappedResult, Threading::WaitForSingleObject},
};

pub(super) struct Reader {
    handle: Handle,
    slot: Box<Slot>,
    pending: bool,
    eof: bool,
    ready: usize,
    offset: usize,
}

impl Reader {
    pub fn new(handle: Handle) -> io::Result<Self> {
        let mut reader = Self {
            handle,
            slot: Slot::new()?,
            pending: false,
            eof: false,
            ready: 0,
            offset: 0,
        };
        reader.arm()?;
        Ok(reader)
    }

    pub fn read(&mut self, buffer: &mut [u8]) -> Result<Transfer, Failure> {
        self.harvest()
            .map_err(|error| Failure::io(Stage::Read, error))?;
        if self.offset < self.ready {
            return Ok(self.take(buffer));
        }
        if self.eof {
            return Ok(Transfer::Eof);
        }
        if !self.pending {
            self.arm()
                .map_err(|error| Failure::io(Stage::Read, error))?;
            if self.offset < self.ready {
                return Ok(self.take(buffer));
            }
            if self.eof {
                return Ok(Transfer::Eof);
            }
        }
        Ok(Transfer::Pending)
    }

    pub fn ready(&self) -> bool {
        self.offset < self.ready
            || (self.pending
                && unsafe { WaitForSingleObject(self.slot.event.raw(), 0) } == WAIT_OBJECT_0)
    }

    pub fn eof(&self) -> bool {
        self.eof
    }

    pub fn event(&self) -> HANDLE {
        self.slot.event.raw()
    }

    fn take(&mut self, buffer: &mut [u8]) -> Transfer {
        let available = self.ready - self.offset;
        let count = buffer.len().min(CHUNK).min(available);
        buffer[..count].copy_from_slice(&self.slot.buffer[self.offset..self.offset + count]);
        self.offset += count;
        if self.offset == self.ready {
            self.offset = 0;
            self.ready = 0;
            let _ = self.arm();
        }
        Transfer::Bytes(count)
    }

    fn arm(&mut self) -> io::Result<()> {
        if self.pending || self.eof {
            return Ok(());
        }
        self.slot.reset()?;
        let mut read = 0u32;
        let ok = unsafe {
            ReadFile(
                self.handle.raw(),
                self.slot.buffer.as_mut_ptr(),
                CHUNK as u32,
                &mut read,
                &mut self.slot.overlapped,
            )
        };
        if ok != 0 {
            if read == 0 {
                self.eof = true;
            } else {
                self.ready = read as usize;
                self.offset = 0;
            }
            return Ok(());
        }
        match code() {
            ERROR_IO_PENDING => {
                self.pending = true;
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
            if transferred == 0 {
                self.eof = true;
            } else {
                self.ready = transferred as usize;
                self.offset = 0;
            }
            return Ok(());
        }
        match code() {
            ERROR_IO_INCOMPLETE => Ok(()),
            ERROR_BROKEN_PIPE | ERROR_PIPE_NOT_CONNECTED | ERROR_HANDLE_EOF => {
                self.pending = false;
                self.eof = true;
                Ok(())
            }
            other => {
                self.pending = false;
                Err(io::Error::from_raw_os_error(other as i32))
            }
        }
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        if self.pending {
            cancel(self.handle.raw(), &self.slot.overlapped);
            self.pending = false;
        }
    }
}
