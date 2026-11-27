use super::{
    handle::{Handle, Security, code},
    security::{canonical, peer, pipename, private, wide},
};
use std::{io, mem::zeroed, path::Path, ptr};
use windows_sys::Win32::{
    Foundation::{
        ERROR_IO_INCOMPLETE, ERROR_IO_PENDING, ERROR_PIPE_CONNECTED, GENERIC_READ, GENERIC_WRITE,
        HANDLE,
    },
    Storage::FileSystem::{
        CREATE_NEW, CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_FLAG_DELETE_ON_CLOSE,
        FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, PIPE_ACCESS_DUPLEX,
    },
    System::{
        IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED},
        Pipes::{
            ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeClientProcessId, PIPE_READMODE_BYTE,
            PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
        },
        Threading::{CreateEventW, ResetEvent},
    },
};

struct Pending {
    overlapped: OVERLAPPED,
    event: Handle,
}

unsafe impl Send for Pending {}
unsafe impl Sync for Pending {}

impl Pending {
    fn new() -> io::Result<Box<Self>> {
        let event = Handle::new(unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) })?;
        let mut overlapped: OVERLAPPED = unsafe { zeroed() };
        overlapped.hEvent = event.raw();
        Ok(Box::new(Self { overlapped, event }))
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

pub(super) struct Socket {
    _marker: Handle,
    listener: Handle,
    pending: Box<Pending>,
    waiting: bool,
    ready: bool,
    name: Vec<u16>,
    security: Security,
}

pub(super) fn namespace(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::Error::other("socket path must be absolute"));
    }
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("socket has no parent"))?;
    if !canonical(parent)? {
        return Err(io::Error::other("socket parent must be canonical"));
    }
    if !private(parent)? {
        return Err(io::Error::other(
            "socket parent must be a private directory owned by this user",
        ));
    }
    Ok(())
}

impl Socket {
    pub fn bind(path: &Path) -> io::Result<Self> {
        namespace(path)?;
        let mut security = Security::private()?;
        let name = pipename(path)?;
        let listener = listen(&name, &mut security, true)?;
        let pending = Pending::new()?;
        let mut socket = Self {
            _marker: marker(path, &mut security)?,
            listener,
            pending,
            waiting: false,
            ready: false,
            name,
            security,
        };
        socket.arm()?;
        Ok(socket)
    }

    pub fn accept(&mut self) -> io::Result<Option<Handle>> {
        if !self.poll()? {
            return Ok(None);
        }
        self.ready = false;
        let next = listen(&self.name, &mut self.security, false)?;
        let accepted = std::mem::replace(&mut self.listener, next);
        self.arm()?;
        let mut pid = 0u32;
        if unsafe { GetNamedPipeClientProcessId(accepted.raw(), &mut pid) } == 0 {
            return Ok(None);
        }
        if peer(pid).is_err() {
            return Ok(None);
        }
        Ok(Some(accepted))
    }

    pub fn ready(&self) -> bool {
        self.ready
    }

    pub fn event(&self) -> Option<HANDLE> {
        self.waiting.then(|| self.pending.event.raw())
    }

    fn poll(&mut self) -> io::Result<bool> {
        if self.ready {
            return Ok(true);
        }
        if !self.waiting {
            self.arm()?;
            return Ok(self.ready);
        }
        let mut transferred = 0u32;
        let ok = unsafe {
            GetOverlappedResult(
                self.listener.raw(),
                &self.pending.overlapped,
                &mut transferred,
                0,
            )
        };
        if ok != 0 {
            self.waiting = false;
            self.ready = true;
            return Ok(true);
        }
        if code() == ERROR_IO_INCOMPLETE {
            return Ok(false);
        }
        self.waiting = false;
        Err(io::Error::last_os_error())
    }

    fn arm(&mut self) -> io::Result<()> {
        self.pending.reset()?;
        let ok = unsafe { ConnectNamedPipe(self.listener.raw(), &mut self.pending.overlapped) };
        if ok != 0 {
            self.ready = true;
            self.waiting = false;
            return Ok(());
        }
        match code() {
            ERROR_IO_PENDING => {
                self.waiting = true;
                self.ready = false;
                Ok(())
            }
            ERROR_PIPE_CONNECTED => {
                self.ready = true;
                self.waiting = false;
                Ok(())
            }
            other => Err(io::Error::from_raw_os_error(other as i32)),
        }
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        if self.waiting {
            unsafe {
                CancelIoEx(self.listener.raw(), &self.pending.overlapped);
                let mut transferred = 0u32;
                GetOverlappedResult(
                    self.listener.raw(),
                    &self.pending.overlapped,
                    &mut transferred,
                    1,
                );
            }
            self.waiting = false;
        }
    }
}

fn marker(path: &Path, security: &mut Security) -> io::Result<Handle> {
    let target = wide(path.as_os_str());
    Handle::new(unsafe {
        CreateFileW(
            target.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ,
            security.attributes(),
            CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_DELETE_ON_CLOSE,
            ptr::null_mut(),
        )
    })
}

fn listen(name: &[u16], security: &mut Security, first: bool) -> io::Result<Handle> {
    let mut mode = PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED;
    if first {
        mode |= FILE_FLAG_FIRST_PIPE_INSTANCE;
    }
    let pipe = PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS;
    Handle::new(unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            mode,
            pipe,
            PIPE_UNLIMITED_INSTANCES,
            65_536,
            65_536,
            0,
            security.attributes(),
        )
    })
}
