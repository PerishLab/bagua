use super::handle::{Handle, Security};
use crate::CHUNK;
use std::{
    io,
    mem::zeroed,
    ptr,
    sync::atomic::{AtomicU64, Ordering},
};
use windows_sys::Win32::{
    Foundation::{ERROR_IO_PENDING, ERROR_PIPE_CONNECTED, HANDLE},
    Security::Cryptography::{BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom},
    Storage::FileSystem::{
        CreateFileW, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, FILE_GENERIC_READ,
        FILE_GENERIC_WRITE, OPEN_EXISTING, PIPE_ACCESS_INBOUND, PIPE_ACCESS_OUTBOUND,
    },
    System::{
        IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED},
        Pipes::{
            ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_TYPE_BYTE, PIPE_WAIT,
        },
        Threading::{CreateEventW, GetCurrentProcessId, ResetEvent},
    },
};

static COUNTER: AtomicU64 = AtomicU64::new(1);

pub(super) fn pair(
    inbound: bool,
    size: u32,
    security: &mut Security,
) -> io::Result<(Handle, Handle)> {
    let name = endpoint()?;
    let mode = if inbound {
        PIPE_ACCESS_INBOUND
    } else {
        PIPE_ACCESS_OUTBOUND
    } | FILE_FLAG_OVERLAPPED
        | FILE_FLAG_FIRST_PIPE_INSTANCE;
    let pipe = PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS;
    let server = Handle::new(unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            mode,
            pipe,
            1,
            size,
            size,
            0,
            security.attributes(),
        )
    })?;
    let event = Handle::new(unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) })?;
    let mut overlapped: OVERLAPPED = unsafe { zeroed() };
    overlapped.hEvent = event.raw();
    let connected = unsafe { ConnectNamedPipe(server.raw(), &mut overlapped) };
    let pending = if connected == 0 {
        match code() {
            ERROR_IO_PENDING => true,
            ERROR_PIPE_CONNECTED => false,
            other => return Err(io::Error::from_raw_os_error(other as i32)),
        }
    } else {
        false
    };
    let access = if inbound {
        FILE_GENERIC_WRITE
    } else {
        FILE_GENERIC_READ
    };
    let client = Handle::new(unsafe {
        CreateFileW(
            name.as_ptr(),
            access,
            0,
            security.attributes(),
            OPEN_EXISTING,
            0,
            ptr::null_mut(),
        )
    })?;
    if pending {
        let mut transferred = 0u32;
        if unsafe { GetOverlappedResult(server.raw(), &overlapped, &mut transferred, 1) } == 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok((server, client))
}

pub(super) struct Slot {
    pub overlapped: OVERLAPPED,
    pub buffer: [u8; CHUNK],
    pub event: Handle,
}

unsafe impl Send for Slot {}
unsafe impl Sync for Slot {}

impl Slot {
    pub fn new() -> io::Result<Box<Self>> {
        let event = Handle::new(unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) })?;
        let mut overlapped: OVERLAPPED = unsafe { zeroed() };
        overlapped.hEvent = event.raw();
        Ok(Box::new(Self {
            overlapped,
            buffer: [0; CHUNK],
            event,
        }))
    }

    pub fn reset(&mut self) -> io::Result<()> {
        if unsafe { ResetEvent(self.event.raw()) } == 0 {
            return Err(io::Error::last_os_error());
        }
        self.overlapped = unsafe { zeroed() };
        self.overlapped.hEvent = self.event.raw();
        Ok(())
    }
}

pub(super) fn cancel(handle: HANDLE, overlapped: &OVERLAPPED) {
    unsafe {
        CancelIoEx(handle, overlapped);
        let mut transferred = 0u32;
        GetOverlappedResult(handle, overlapped, &mut transferred, 1);
    }
}

pub(super) fn code() -> u32 {
    io::Error::last_os_error().raw_os_error().unwrap_or(0) as u32
}

fn endpoint() -> io::Result<Vec<u16>> {
    let mut bytes = [0u8; 8];
    let status = unsafe {
        BCryptGenRandom(
            ptr::null_mut(),
            bytes.as_mut_ptr(),
            bytes.len() as u32,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    if status != 0 {
        return Err(io::Error::other("system entropy unavailable"));
    }
    let pid = unsafe { GetCurrentProcessId() };
    let sequence = COUNTER.fetch_add(1, Ordering::Relaxed);
    let token = u64::from_le_bytes(bytes);
    let name = format!(r"\\.\pipe\bagua-pty-{pid}-{sequence}-{token:016x}");
    let mut wide: Vec<u16> = name.encode_utf16().collect();
    wide.push(0);
    Ok(wide)
}
