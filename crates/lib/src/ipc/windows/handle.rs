use std::{ffi::c_void, io, mem::size_of, ptr};
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, DUPLICATE_SAME_ACCESS, DuplicateHandle, ERROR_INSUFFICIENT_BUFFER, HANDLE,
        INVALID_HANDLE_VALUE, LocalFree,
    },
    Security::{
        ACL,
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            SDDL_REVISION_1,
        },
        GetSecurityDescriptorDacl, GetTokenInformation, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES,
        TOKEN_QUERY, TOKEN_USER, TokenUser,
    },
    System::Threading::{GetCurrentProcess, OpenProcessToken},
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

    pub fn clone(&self) -> io::Result<Self> {
        let mut copy = ptr::null_mut();
        let current = unsafe { GetCurrentProcess() };
        let ok = unsafe {
            DuplicateHandle(
                current,
                self.0,
                current,
                &mut copy,
                0,
                0,
                DUPLICATE_SAME_ACCESS,
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Self::new(copy)
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

pub(super) struct Local(pub *mut c_void);

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
    descriptor: Local,
    attributes: SECURITY_ATTRIBUTES,
}

unsafe impl Send for Security {}
unsafe impl Sync for Security {}

impl Security {
    pub fn private() -> io::Result<Self> {
        let (_, sid) = user(unsafe { GetCurrentProcess() })?;
        let sddl = format!("D:P(A;;GA;;;{sid})\0");
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
            descriptor: Local(descriptor),
            attributes,
        })
    }

    pub fn attributes(&mut self) -> *mut SECURITY_ATTRIBUTES {
        &mut self.attributes
    }

    pub fn dacl(&self) -> io::Result<*mut ACL> {
        let mut present = 0;
        let mut defaulted = 0;
        let mut dacl = ptr::null_mut();
        let ok = unsafe {
            GetSecurityDescriptorDacl(self.descriptor.0, &mut present, &mut dacl, &mut defaulted)
        };
        if ok == 0 || present == 0 || dacl.is_null() {
            return Err(io::Error::last_os_error());
        }
        Ok(dacl)
    }
}

pub(super) fn code() -> u32 {
    io::Error::last_os_error().raw_os_error().unwrap_or(0) as u32
}

pub(super) fn user(process: HANDLE) -> io::Result<(Vec<u8>, String)> {
    let mut raw = ptr::null_mut();
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut raw) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let token = Handle::new(raw)?;
    let mut needed = 0u32;
    let first =
        unsafe { GetTokenInformation(token.raw(), TokenUser, ptr::null_mut(), 0, &mut needed) };
    if first == 0 && code() != ERROR_INSUFFICIENT_BUFFER {
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
    let sid = unsafe { (*(buffer.as_ptr().cast::<TOKEN_USER>())).User.Sid };
    let mut text = ptr::null_mut();
    if unsafe { ConvertSidToStringSidW(sid, &mut text) } == 0 || text.is_null() {
        return Err(io::Error::last_os_error());
    }
    let _guard = Local(text.cast());
    let mut length = 0;
    while unsafe { *text.add(length) } != 0 {
        length += 1;
    }
    let slice = unsafe { std::slice::from_raw_parts(text, length) };
    let string = String::from_utf16(slice).map_err(|_| io::Error::other("invalid user SID"))?;
    Ok((buffer, string))
}
