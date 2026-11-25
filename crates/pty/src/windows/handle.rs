use std::{ffi::c_void, io, mem::size_of, ptr};
use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_INSUFFICIENT_BUFFER, HANDLE, INVALID_HANDLE_VALUE, LocalFree},
    Security::{
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            SDDL_REVISION_1,
        },
        GetTokenInformation, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
        TokenUser,
    },
    System::{
        Console::{ClosePseudoConsole, HPCON},
        Threading::{GetCurrentProcess, OpenProcessToken},
    },
};

pub(super) struct Handle(HANDLE);

unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

impl Handle {
    pub fn new(raw: HANDLE) -> io::Result<Self> {
        if raw.is_null() || raw == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(raw))
    }

    pub fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

pub(super) struct Console(HPCON);

unsafe impl Send for Console {}
unsafe impl Sync for Console {}

impl Console {
    pub fn new(raw: HPCON) -> Self {
        Self(raw)
    }

    pub fn raw(&self) -> HPCON {
        self.0
    }
}

impl Drop for Console {
    fn drop(&mut self) {
        if self.0 != 0 {
            unsafe {
                ClosePseudoConsole(self.0);
            }
        }
    }
}

struct Local(*mut c_void);

impl Drop for Local {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                LocalFree(self.0);
            }
        }
    }
}

pub(super) struct Security {
    _descriptor: Local,
    attributes: SECURITY_ATTRIBUTES,
}

impl Security {
    pub fn private() -> io::Result<Self> {
        let sddl = format!("D:P(A;;GA;;;{})\0", owner()?);
        let wide: Vec<u16> = sddl.encode_utf16().collect();
        let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                wide.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                ptr::null_mut(),
            )
        };
        if ok == 0 || descriptor.is_null() {
            return Err(io::Error::last_os_error());
        }
        let attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor,
            bInheritHandle: 0,
        };
        Ok(Self {
            _descriptor: Local(descriptor),
            attributes,
        })
    }

    pub fn attributes(&mut self) -> *mut SECURITY_ATTRIBUTES {
        &mut self.attributes
    }
}

fn owner() -> io::Result<String> {
    let mut token = ptr::null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let token = Handle::new(token)?;
    let mut needed = 0u32;
    let first =
        unsafe { GetTokenInformation(token.raw(), TokenUser, ptr::null_mut(), 0, &mut needed) };
    if first == 0
        && io::Error::last_os_error().raw_os_error() != Some(ERROR_INSUFFICIENT_BUFFER as i32)
    {
        return Err(io::Error::last_os_error());
    }
    let mut buffer = vec![0u8; needed as usize];
    let ok = unsafe {
        GetTokenInformation(
            token.raw(),
            TokenUser,
            buffer.as_mut_ptr().cast(),
            needed,
            &mut needed,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let user = unsafe { &*(buffer.as_ptr().cast::<TOKEN_USER>()) };
    let mut text = ptr::null_mut();
    if unsafe { ConvertSidToStringSidW(user.User.Sid, &mut text) } == 0 || text.is_null() {
        return Err(io::Error::last_os_error());
    }
    let guard = Local(text.cast());
    let mut length = 0;
    while unsafe { *text.add(length) } != 0 {
        length += 1;
    }
    let slice = unsafe { std::slice::from_raw_parts(text, length) };
    let result = String::from_utf16(slice).map_err(|_| io::Error::other("invalid user SID"));
    drop(guard);
    result
}
