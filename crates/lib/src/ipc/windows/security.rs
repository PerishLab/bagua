use super::handle::{Handle, Local, Security, user};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsStr,
    fs, io,
    mem::{size_of, zeroed},
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    ptr,
};
use windows_sys::Win32::{
    Foundation::ERROR_SUCCESS,
    Security::{
        ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, ACL_SIZE_INFORMATION, AclSizeInformation,
        Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT, SetNamedSecurityInfoW},
        Cryptography::{BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom},
        DACL_SECURITY_INFORMATION, EqualSid, GetAce, GetAclInformation, OWNER_SECURITY_INFORMATION,
        PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, TOKEN_USER,
    },
    System::Threading::{GetCurrentProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
};

const ALLOWED: u8 = 0;

pub(super) fn wide(value: &OsStr) -> Vec<u16> {
    let mut units: Vec<u16> = value.encode_wide().collect();
    units.push(0);
    units
}

pub(super) fn random(buffer: &mut [u8]) -> io::Result<()> {
    let status = unsafe {
        BCryptGenRandom(
            ptr::null_mut(),
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    if status != 0 {
        return Err(io::Error::other("host context entropy unavailable"));
    }
    Ok(())
}

pub(super) fn strip(path: &Path) -> PathBuf {
    let Some(text) = path.to_str() else {
        return path.to_path_buf();
    };
    if let Some(rest) = text.strip_prefix(r"\\?\")
        && !rest.starts_with("UNC\\")
    {
        return PathBuf::from(rest);
    }
    path.to_path_buf()
}

pub(super) fn canonical(directory: &Path) -> io::Result<bool> {
    let resolved = fs::canonicalize(directory)?;
    Ok(strip(&resolved) == strip(directory))
}

pub fn secure(directory: &Path) -> io::Result<()> {
    let security = Security::private()?;
    let dacl = security.dacl()?;
    let (token, _) = user(unsafe { GetCurrentProcess() })?;
    let current = unsafe { (*(token.as_ptr().cast::<TOKEN_USER>())).User.Sid };
    let mut target = wide(directory.as_os_str());
    let flags = OWNER_SECURITY_INFORMATION
        | DACL_SECURITY_INFORMATION
        | PROTECTED_DACL_SECURITY_INFORMATION;
    let status = unsafe {
        SetNamedSecurityInfoW(
            target.as_mut_ptr(),
            SE_FILE_OBJECT,
            flags,
            current,
            ptr::null_mut(),
            dacl,
            ptr::null_mut(),
        )
    };
    if status != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    Ok(())
}

pub fn private(directory: &Path) -> io::Result<bool> {
    if !canonical(directory)? {
        return Ok(false);
    }
    let metadata = fs::symlink_metadata(directory)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Ok(false);
    }
    let (token, _) = user(unsafe { GetCurrentProcess() })?;
    let current = unsafe { (*(token.as_ptr().cast::<TOKEN_USER>())).User.Sid };
    let target = wide(directory.as_os_str());
    let mut owner: PSID = ptr::null_mut();
    let mut dacl: *mut ACL = ptr::null_mut();
    let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
    let status = unsafe {
        GetNamedSecurityInfoW(
            target.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            ptr::null_mut(),
            &mut dacl,
            ptr::null_mut(),
            &mut descriptor,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    let _guard = Local(descriptor);
    if owner.is_null() || dacl.is_null() || unsafe { EqualSid(owner, current) } == 0 {
        return Ok(false);
    }
    exclusive(dacl, current)
}

pub(super) fn peer(pid: u32) -> io::Result<()> {
    let process = Handle::new(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) })?;
    let (other, _) = user(process.raw())?;
    let (mine, _) = user(unsafe { GetCurrentProcess() })?;
    let left = unsafe { (*(other.as_ptr().cast::<TOKEN_USER>())).User.Sid };
    let right = unsafe { (*(mine.as_ptr().cast::<TOKEN_USER>())).User.Sid };
    if unsafe { EqualSid(left, right) } == 0 {
        return Err(io::Error::other("socket peer belongs to another user"));
    }
    Ok(())
}

pub(super) fn pipename(path: &Path) -> io::Result<Vec<u16>> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("socket has no parent"))?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::other("socket has no name"))?;
    let resolved = strip(&fs::canonicalize(parent)?).join(name);
    let normalized = resolved
        .to_str()
        .ok_or_else(|| io::Error::other("socket path must be UTF-8"))?
        .to_lowercase();
    let (_, sid) = user(unsafe { GetCurrentProcess() })?;
    let mut hasher = Sha256::new();
    hasher.update(sid.as_bytes());
    hasher.update([0]);
    hasher.update(normalized.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut hex, "{byte:02x}");
    }
    Ok(wide(OsStr::new(&format!(r"\\.\pipe\bagua-v1-{hex}"))))
}

fn exclusive(dacl: *mut ACL, current: PSID) -> io::Result<bool> {
    let mut info: ACL_SIZE_INFORMATION = unsafe { zeroed() };
    let ok = unsafe {
        GetAclInformation(
            dacl,
            (&mut info as *mut ACL_SIZE_INFORMATION).cast(),
            size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut granted = false;
    for index in 0..info.AceCount {
        let mut ace = ptr::null_mut();
        if unsafe { GetAce(dacl, index, &mut ace) } == 0 || ace.is_null() {
            return Err(io::Error::last_os_error());
        }
        let header = unsafe { &*(ace.cast::<ACE_HEADER>()) };
        if header.AceType == ALLOWED {
            let allowed = unsafe { &*(ace.cast::<ACCESS_ALLOWED_ACE>()) };
            let sid = ptr::addr_of!(allowed.SidStart) as PSID;
            if unsafe { EqualSid(sid, current) } == 0 {
                return Ok(false);
            }
            granted = true;
        }
    }
    Ok(granted)
}
