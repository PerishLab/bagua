use super::{
    handle::{Handle, code},
    security::{peer, pipename},
    socket::namespace,
};
use protocol::{
    Request,
    wire::{self, Message},
};
use std::{
    fs, io,
    mem::zeroed,
    path::Path,
    ptr,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{
        ERROR_BROKEN_PIPE, ERROR_FILE_NOT_FOUND, ERROR_HANDLE_EOF, ERROR_IO_PENDING,
        ERROR_PIPE_BUSY, ERROR_PIPE_NOT_CONNECTED, GENERIC_READ, GENERIC_WRITE, HANDLE,
        WAIT_OBJECT_0, WAIT_TIMEOUT,
    },
    Storage::FileSystem::{
        CreateFileW, FILE_FLAG_OVERLAPPED, FlushFileBuffers, OPEN_EXISTING, ReadFile,
        SECURITY_IDENTIFICATION, SECURITY_SQOS_PRESENT, WriteFile,
    },
    System::{
        IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED},
        Pipes::{GetNamedPipeServerProcessId, WaitNamedPipeW},
        Threading::{CreateEventW, INFINITE, WaitForSingleObject},
    },
};

pub struct Connection {
    handle: Mutex<Option<Handle>>,
    timeout: AtomicU64,
}

impl Connection {
    pub fn channel(self) -> io::Result<super::Channel> {
        let handle = self
            .handle
            .into_inner()
            .map_err(|_| io::Error::other("connection lock poisoned"))?
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "connection closed"))?;
        super::Channel::new(handle)
    }

    pub fn connect(path: &Path) -> io::Result<Self> {
        namespace(path)?;
        let metadata = fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink() {
            return Err(io::Error::other("socket endpoint must not be a symlink"));
        }
        let name = pipename(path)?;
        let handle = open(&name)?;
        let mut pid = 0u32;
        if unsafe { GetNamedPipeServerProcessId(handle.raw(), &mut pid) } == 0 {
            return Err(io::Error::last_os_error());
        }
        peer(pid)?;
        Ok(Self {
            handle: Mutex::new(Some(handle)),
            timeout: AtomicU64::new(10_000),
        })
    }

    pub fn send(&mut self, request: &Request) -> io::Result<()> {
        let bytes = wire::encode(request, wire::REQUEST).map_err(crate::ipc::error)?;
        let guard = self
            .handle
            .lock()
            .map_err(|_| io::Error::other("connection lock poisoned"))?;
        let handle = guard
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "connection closed"))?;
        let limit = self.limit();
        let mut offset = 0;
        while offset < bytes.len() {
            let chunk = (bytes.len() - offset).min(65_536);
            let written = write(handle.raw(), &bytes[offset..offset + chunk], limit)?;
            if written == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "host connection closed",
                ));
            }
            offset += written;
        }
        Ok(())
    }

    pub fn receive(&mut self) -> io::Result<Message> {
        let mut header = [0u8; 4];
        self.exact(&mut header)?;
        let length = u32::from_be_bytes(header) as usize;
        if length == 0 || length > wire::FRAME {
            return Err(crate::ipc::error(protocol::Failure::Payload));
        }
        let mut bytes = vec![0u8; length];
        self.exact(&mut bytes)?;
        serde_json::from_slice(&bytes).map_err(io::Error::other)
    }

    pub fn finish(&self) -> io::Result<()> {
        let mut guard = self
            .handle
            .lock()
            .map_err(|_| io::Error::other("connection lock poisoned"))?;
        if let Some(handle) = guard.take() {
            unsafe {
                FlushFileBuffers(handle.raw());
            }
            drop(handle);
        }
        Ok(())
    }

    pub fn duplicate(&self) -> io::Result<Self> {
        let guard = self
            .handle
            .lock()
            .map_err(|_| io::Error::other("connection lock poisoned"))?;
        let copy = guard
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "connection closed"))?
            .clone()?;
        Ok(Self {
            handle: Mutex::new(Some(copy)),
            timeout: AtomicU64::new(self.timeout.load(Ordering::Relaxed)),
        })
    }

    pub fn streaming(&self) -> io::Result<()> {
        self.timeout.store(0, Ordering::Relaxed);
        Ok(())
    }

    pub fn bounded(&self, timeout: Duration) -> io::Result<()> {
        let millis = timeout.as_millis().clamp(1, u64::MAX as u128) as u64;
        self.timeout.store(millis, Ordering::Relaxed);
        Ok(())
    }

    fn exact(&self, target: &mut [u8]) -> io::Result<()> {
        let guard = self
            .handle
            .lock()
            .map_err(|_| io::Error::other("connection lock poisoned"))?;
        let handle = guard
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "connection closed"))?;
        let limit = self.limit();
        let mut offset = 0;
        while offset < target.len() {
            let count = read(handle.raw(), &mut target[offset..], limit)?;
            if count == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "host connection closed",
                ));
            }
            offset += count;
        }
        Ok(())
    }

    fn limit(&self) -> u32 {
        match self.timeout.load(Ordering::Relaxed) {
            0 => INFINITE,
            millis => millis.min(u32::MAX as u64 - 1) as u32,
        }
    }
}

fn open(name: &[u16]) -> io::Result<Handle> {
    let flags = FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION;
    for _ in 0..8 {
        let raw = unsafe {
            CreateFileW(
                name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                ptr::null(),
                OPEN_EXISTING,
                flags,
                ptr::null_mut(),
            )
        };
        if let Ok(handle) = Handle::new(raw) {
            return Ok(handle);
        }
        if !retry(name)? {
            break;
        }
    }
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "host pipe remained busy",
    ))
}

fn retry(name: &[u16]) -> io::Result<bool> {
    match code() {
        ERROR_PIPE_BUSY => {
            unsafe {
                WaitNamedPipeW(name.as_ptr(), 250);
            }
            Ok(true)
        }
        ERROR_FILE_NOT_FOUND => Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            "endpoint has no active host",
        )),
        other => Err(io::Error::from_raw_os_error(other as i32)),
    }
}

fn read(handle: HANDLE, target: &mut [u8], limit: u32) -> io::Result<usize> {
    let event = Handle::new(unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) })?;
    let mut overlapped: OVERLAPPED = unsafe { zeroed() };
    overlapped.hEvent = event.raw();
    let mut count = 0u32;
    let ok = unsafe {
        ReadFile(
            handle,
            target.as_mut_ptr(),
            target.len().min(65_536) as u32,
            &mut count,
            &mut overlapped,
        )
    };
    if ok != 0 {
        return Ok(count as usize);
    }
    match code() {
        ERROR_IO_PENDING => wait(handle, &overlapped, event.raw(), limit),
        ERROR_BROKEN_PIPE | ERROR_PIPE_NOT_CONNECTED | ERROR_HANDLE_EOF => Ok(0),
        other => Err(io::Error::from_raw_os_error(other as i32)),
    }
}

fn write(handle: HANDLE, data: &[u8], limit: u32) -> io::Result<usize> {
    let event = Handle::new(unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) })?;
    let mut overlapped: OVERLAPPED = unsafe { zeroed() };
    overlapped.hEvent = event.raw();
    let mut count = 0u32;
    let ok = unsafe {
        WriteFile(
            handle,
            data.as_ptr(),
            data.len() as u32,
            &mut count,
            &mut overlapped,
        )
    };
    if ok != 0 {
        return Ok(count as usize);
    }
    match code() {
        ERROR_IO_PENDING => wait(handle, &overlapped, event.raw(), limit),
        ERROR_BROKEN_PIPE | ERROR_PIPE_NOT_CONNECTED => Ok(0),
        other => Err(io::Error::from_raw_os_error(other as i32)),
    }
}

fn wait(handle: HANDLE, overlapped: &OVERLAPPED, event: HANDLE, limit: u32) -> io::Result<usize> {
    match unsafe { WaitForSingleObject(event, limit) } {
        WAIT_OBJECT_0 => {
            let mut count = 0u32;
            if unsafe { GetOverlappedResult(handle, overlapped, &mut count, 0) } != 0 {
                return Ok(count as usize);
            }
            match code() {
                ERROR_BROKEN_PIPE | ERROR_PIPE_NOT_CONNECTED | ERROR_HANDLE_EOF => Ok(0),
                other => Err(io::Error::from_raw_os_error(other as i32)),
            }
        }
        WAIT_TIMEOUT => {
            unsafe {
                CancelIoEx(handle, overlapped);
                let mut count = 0u32;
                GetOverlappedResult(handle, overlapped, &mut count, 1);
            }
            Err(io::Error::new(io::ErrorKind::TimedOut, "IPC IO timed out"))
        }
        _ => Err(io::Error::last_os_error()),
    }
}
