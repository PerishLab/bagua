use super::super::handle::Handle;
use std::{
    ffi::c_void,
    io,
    mem::{size_of, zeroed},
    path::{Path, PathBuf},
    ptr,
};
use windows_sys::{
    Wdk::System::Threading::{NtQueryInformationProcess, ProcessBasicInformation},
    Win32::{
        Foundation::{ERROR_INSUFFICIENT_BUFFER, FILETIME, HANDLE, UNICODE_STRING, WAIT_TIMEOUT},
        Security::{EqualSid, GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser},
        System::{
            Diagnostics::Debug::ReadProcessMemory,
            Threading::{
                GetCurrentProcess, GetProcessTimes, OpenProcess, OpenProcessToken, PEB,
                PROCESS_BASIC_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
                PROCESS_VM_READ, QueryFullProcessImageNameW, WaitForSingleObject,
            },
        },
    },
};

const OFFSET: usize = 0x38;
const LIMIT: usize = 4096;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Stamp {
    pub pid: u32,
    pub started: u64,
}

pub(super) struct Target {
    handle: Handle,
    pid: u32,
}

impl Target {
    pub fn open(pid: u32) -> io::Result<Self> {
        if pid == 0 {
            return Err(io::Error::other("invalid process identity"));
        }
        let access = PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ | PROCESS_SYNCHRONIZE;
        let handle = Handle::new(unsafe { OpenProcess(access, 0, pid) })?;
        same(handle.raw())?;
        alive(handle.raw())?;
        Ok(Self { handle, pid })
    }

    pub fn stamp(&self) -> io::Result<Stamp> {
        alive(self.handle.raw())?;
        let mut created: FILETIME = unsafe { zeroed() };
        let mut exited: FILETIME = unsafe { zeroed() };
        let mut kernel: FILETIME = unsafe { zeroed() };
        let mut user: FILETIME = unsafe { zeroed() };
        let ok = unsafe {
            GetProcessTimes(
                self.handle.raw(),
                &mut created,
                &mut exited,
                &mut kernel,
                &mut user,
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        let started = ((created.dwHighDateTime as u64) << 32) | (created.dwLowDateTime as u64);
        if started == 0 {
            return Err(io::Error::other("missing process lifetime"));
        }
        Ok(Stamp {
            pid: self.pid,
            started,
        })
    }

    pub fn command(&self) -> io::Result<String> {
        let mut capacity = 512usize;
        loop {
            let mut buffer = vec![0u16; capacity];
            let mut length = buffer.len() as u32;
            let ok = unsafe {
                QueryFullProcessImageNameW(self.handle.raw(), 0, buffer.as_mut_ptr(), &mut length)
            };
            if ok != 0 {
                let full = String::from_utf16(&buffer[..length as usize])
                    .map_err(|_| io::Error::other("invalid process image path"))?;
                let leaf = Path::new(&full)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| io::Error::other("missing process command"))?;
                let name = stem(leaf);
                if name.is_empty() || name.len() > 256 {
                    return Err(io::Error::other("process command exceeds its bound"));
                }
                return Ok(name.into());
            }
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(ERROR_INSUFFICIENT_BUFFER as i32) && capacity < LIMIT {
                capacity *= 2;
                continue;
            }
            return Err(error);
        }
    }

    pub fn cwd(&self) -> io::Result<PathBuf> {
        let mut basic: PROCESS_BASIC_INFORMATION = unsafe { zeroed() };
        let mut returned = 0u32;
        let status = unsafe {
            NtQueryInformationProcess(
                self.handle.raw(),
                ProcessBasicInformation,
                ptr::from_mut(&mut basic).cast(),
                size_of::<PROCESS_BASIC_INFORMATION>() as u32,
                &mut returned,
            )
        };
        if status != 0 || basic.PebBaseAddress.is_null() {
            return Err(io::Error::other("unable to query target PEB"));
        }
        let peb: PEB = read(self.handle.raw(), basic.PebBaseAddress.cast())?;
        if peb.ProcessParameters.is_null() {
            return Err(io::Error::other("missing target process parameters"));
        }
        let pointer = (peb.ProcessParameters as usize)
            .checked_add(OFFSET)
            .ok_or_else(|| io::Error::other("invalid target process parameters"))?;
        let dos: UNICODE_STRING = read(self.handle.raw(), pointer as *const c_void)?;
        let bytes = dos.Length as usize;
        if bytes == 0 || dos.Buffer.is_null() {
            return Err(io::Error::other("foreground cwd is missing"));
        }
        if !bytes.is_multiple_of(2) || bytes > LIMIT * 2 {
            return Err(io::Error::other("foreground cwd exceeds its bound"));
        }
        let utf16 = words(self.handle.raw(), dos.Buffer.cast(), bytes / 2)?;
        let decoded = String::from_utf16(&utf16)
            .map_err(|_| io::Error::other("foreground cwd must be UTF-8"))?;
        normalize(decoded)
    }
}

pub(super) fn alive(raw: HANDLE) -> io::Result<()> {
    if unsafe { WaitForSingleObject(raw, 0) } != WAIT_TIMEOUT {
        return Err(io::Error::other("foreground process has exited"));
    }
    Ok(())
}

fn same(target: HANDLE) -> io::Result<()> {
    let local = token(unsafe { GetCurrentProcess() })?;
    let remote = token(target)?;
    let mine = unsafe { &*(local.as_ptr().cast::<TOKEN_USER>()) };
    let theirs = unsafe { &*(remote.as_ptr().cast::<TOKEN_USER>()) };
    if unsafe { EqualSid(mine.User.Sid, theirs.User.Sid) } == 0 {
        return Err(io::Error::other(
            "foreground process belongs to another user",
        ));
    }
    Ok(())
}

fn token(process: HANDLE) -> io::Result<Vec<u8>> {
    let mut raw = ptr::null_mut();
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut raw) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let opened = Handle::new(raw)?;
    let mut needed = 0u32;
    let first =
        unsafe { GetTokenInformation(opened.raw(), TokenUser, ptr::null_mut(), 0, &mut needed) };
    if first == 0
        && io::Error::last_os_error().raw_os_error() != Some(ERROR_INSUFFICIENT_BUFFER as i32)
    {
        return Err(io::Error::last_os_error());
    }
    let mut buffer = vec![0u8; needed as usize];
    let ok = unsafe {
        GetTokenInformation(
            opened.raw(),
            TokenUser,
            buffer.as_mut_ptr().cast(),
            needed,
            &mut needed,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(buffer)
}

fn read<T: Copy>(source: HANDLE, address: *const c_void) -> io::Result<T> {
    let mut value: T = unsafe { zeroed() };
    let mut count = 0usize;
    let ok = unsafe {
        ReadProcessMemory(
            source,
            address,
            ptr::from_mut(&mut value).cast(),
            size_of::<T>(),
            &mut count,
        )
    };
    if ok == 0 || count != size_of::<T>() {
        return Err(io::Error::last_os_error());
    }
    Ok(value)
}

fn words(handle: HANDLE, address: *const c_void, count: usize) -> io::Result<Vec<u16>> {
    let mut buffer = vec![0u16; count];
    let expected = count
        .checked_mul(size_of::<u16>())
        .ok_or_else(|| io::Error::other("foreground cwd exceeds its bound"))?;
    let mut got = 0usize;
    let ok = unsafe {
        ReadProcessMemory(
            handle,
            address,
            buffer.as_mut_ptr().cast(),
            expected,
            &mut got,
        )
    };
    if ok == 0 || got != expected {
        return Err(io::Error::last_os_error());
    }
    Ok(buffer)
}

pub(super) fn stem(text: &str) -> &str {
    if text.len() > 4 && text[text.len() - 4..].eq_ignore_ascii_case(".exe") {
        return &text[..text.len() - 4];
    }
    text
}

fn normalize(raw: String) -> io::Result<PathBuf> {
    let mut clean = if let Some(rest) = raw.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = raw.strip_prefix(r"\\?\") {
        rest.into()
    } else {
        raw
    };
    while clean.len() > 3 && clean.ends_with('\\') {
        clean.pop();
    }
    if clean.is_empty() || clean.len() > LIMIT {
        return Err(io::Error::other("foreground cwd exceeds its bound"));
    }
    let path = PathBuf::from(clean);
    if !path.is_absolute() {
        return Err(io::Error::other("foreground cwd must be absolute UTF-8"));
    }
    if !std::fs::metadata(&path)?.is_dir() {
        return Err(io::Error::other("foreground cwd is not a directory"));
    }
    Ok(path)
}
