use std::{
    io::{self, Write},
    ptr::null_mut,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE},
    Storage::FileSystem::{
        CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING, WriteFile,
    },
    System::Console::{GetConsoleMode, SetConsoleMode},
};

pub(super) struct Handle(pub HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

pub(super) struct State {
    pub input: u32,
    pub output: u32,
    pub incp: u32,
    pub outcp: u32,
}

pub(super) struct Writer(pub HANDLE);

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let limit = boundary(buf);
        let mut written = 0u32;
        let ok = unsafe {
            WriteFile(
                self.0,
                buf.as_ptr(),
                limit.min(u32::MAX as usize) as u32,
                &mut written,
                null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(written as usize)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn open(name: &str) -> io::Result<Handle> {
    let mut wide: Vec<u16> = name.encode_utf16().collect();
    wide.push(0);
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null_mut(),
            OPEN_EXISTING,
            0,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    Ok(Handle(handle))
}

pub(super) fn mode(handle: HANDLE) -> io::Result<u32> {
    let mut current = 0u32;
    if unsafe { GetConsoleMode(handle, &mut current) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(current)
}

pub(super) fn apply(handle: HANDLE, value: u32) -> io::Result<()> {
    if unsafe { SetConsoleMode(handle, value) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub(super) fn boundary(bytes: &[u8]) -> usize {
    let start = bytes.len().saturating_sub(3);
    for index in (start..bytes.len()).rev() {
        let byte = bytes[index];
        if byte & 0xC0 == 0x80 {
            continue;
        }
        let expected = match byte {
            0x00..=0x7F => 1,
            0xC2..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF4 => 4,
            _ => 1,
        };
        return if index + expected > bytes.len() && index > 0 {
            index
        } else {
            bytes.len()
        };
    }
    bytes.len()
}

#[cfg(test)]
mod checks {
    use super::boundary;

    #[test]
    fn split() {
        let text = "abc你好";
        let bytes = text.as_bytes();
        assert_eq!(boundary(bytes), bytes.len());
        assert_eq!(boundary(&bytes[..4]), 3);
        assert_eq!(boundary(&bytes[..5]), 3);
        assert_eq!(boundary(&bytes[..6]), 6);
        assert_eq!(boundary(&bytes[3..4]), 1);
    }
}
