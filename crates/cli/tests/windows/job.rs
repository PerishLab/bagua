use super::peer;
use pty::Size;
use std::{
    io,
    mem::size_of_val,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    path::Path,
    ptr,
};
use windows_sys::Win32::{
    Foundation::HANDLE,
    System::{
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_BREAKAWAY_OK,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JobObjectExtendedLimitInformation, SetInformationJobObject,
        },
        Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE},
    },
};

fn owned(raw: HANDLE) -> OwnedHandle {
    assert!(!raw.is_null(), "{}", io::Error::last_os_error());
    unsafe { OwnedHandle::from_raw_handle(raw) }
}

fn login(pid: u32, flags: u32) -> OwnedHandle {
    let job = owned(unsafe { CreateJobObjectW(ptr::null(), ptr::null()) });
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | flags;
    assert_ne!(
        unsafe {
            SetInformationJobObject(
                job.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&raw const limits).cast(),
                size_of_val(&limits) as u32,
            )
        },
        0,
        "{}",
        io::Error::last_os_error()
    );
    let shell = owned(unsafe { OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid) });
    assert_ne!(
        unsafe { AssignProcessToJobObject(job.as_raw_handle(), shell.as_raw_handle()) },
        0,
        "{}",
        io::Error::last_os_error()
    );
    job
}

fn enter(home: &Path, size: Size, flags: u32) -> (OwnedHandle, peer::Peer) {
    let mut shell = peer::Peer::launch(home, peer::comspec().into(), &["/D", "/Q"], size);
    let job = login(shell.id(), flags);
    let exe = env!("CARGO_BIN_EXE_bagua");
    shell.send(format!("\"{exe}\" enter work & exit\r").as_bytes());
    (job, shell)
}

pub fn session(home: &Path, size: Size) -> (OwnedHandle, peer::Peer) {
    enter(home, size, JOB_OBJECT_LIMIT_BREAKAWAY_OK)
}

#[test]
fn lifetime() {
    let home = tempfile::tempdir().unwrap();
    let size = Size::new(120, 30).unwrap();
    let socket = home.path().join("bagua/runtime/host.sock");
    let (job, mut shell) = session(home.path(), size);
    shell.until("PS ");
    shell.send(b"$saved = 318; Write-Output ($saved + 1)\r");
    shell.until("319");
    drop(job);
    shell.finish();

    let mut next = peer::Peer::spawn(
        home.path(),
        &["--no-config", "enter", "--existing", "work"],
        size,
    );
    next.until("PS ");
    next.send(b"Write-Output ($saved + 2)\r");
    next.until("320");
    next.send(b"\x07d");
    assert_eq!(next.finish(), 0);
    peer::shutdown(&socket);
}

#[test]
fn refusal() {
    let home = tempfile::tempdir().unwrap();
    let size = Size::new(120, 30).unwrap();
    let socket = home.path().join("bagua/runtime/host.sock");
    let (_job, mut shell) = enter(home.path(), size, 0);
    shell.until("permits breakaway");
    assert_ne!(shell.finish(), 0);
    assert!(!socket.exists());
}
