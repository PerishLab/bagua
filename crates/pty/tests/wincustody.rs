#![cfg(all(target_os = "windows", target_arch = "x86_64"))]

use pty::{Failure, Launch, Process, Size, Stage};
use radicula_process::Environment;
use std::{ffi::OsString, path::PathBuf};
use windows_sys::Win32::{
    Foundation::{CloseHandle, WAIT_OBJECT_0},
    Storage::FileSystem::SYNCHRONIZE,
    System::Threading::{
        GetCurrentProcess, GetProcessHandleCount, OpenProcess, WaitForSingleObject,
    },
};

fn script(body: &str) -> Launch {
    let root = std::env::var_os("SystemRoot")
        .or_else(|| std::env::var_os("WINDIR"))
        .unwrap_or_else(|| OsString::from(r"C:\Windows"));
    let program = PathBuf::from(root)
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    let mut pairs: Vec<(OsString, OsString)> = Vec::new();
    for key in [
        "SYSTEMROOT",
        "WINDIR",
        "COMSPEC",
        "PATH",
        "PATHEXT",
        "TEMP",
        "TMP",
    ] {
        if let Some(value) = std::env::var_os(key) {
            pairs.push((OsString::from(key), value));
        }
    }
    let mut launch = Launch::new(
        program,
        std::env::temp_dir(),
        Environment::new(pairs),
        Size::new(80, 24).unwrap(),
    );
    launch.args = vec![
        "-NoLogo".into(),
        "-NoProfile".into(),
        "-NonInteractive".into(),
        "-Command".into(),
        body.into(),
    ];
    launch
}

fn handles() -> u32 {
    let mut count = 0u32;
    let ok = unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut count) };
    assert_ne!(ok, 0);
    count
}

fn reaped(pid: u32) {
    let raw = unsafe { OpenProcess(SYNCHRONIZE, 0, pid) };
    if raw.is_null() {
        return;
    }
    let waited = unsafe { WaitForSingleObject(raw, 2_000) };
    unsafe {
        CloseHandle(raw);
    }
    assert_eq!(waited, WAIT_OBJECT_0, "child process {pid} still alive");
}

#[test]
fn custody() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("absent");
    let warmup = Process::spawn(script("exit 0")).unwrap();
    let pid = warmup.id();
    drop(warmup);
    reaped(pid);
    let _ = std::panic::catch_unwind(|| panic!("warmup"));
    let mut warm = script("exit 0");
    warm.program = missing.clone();
    let _ = Process::spawn(warm);
    let baseline = handles();
    for _ in 0..8 {
        let mut launch = script("exit 0");
        launch.program = missing.clone();
        assert!(matches!(
            Process::spawn(launch),
            Err(Failure::Io {
                stage: Stage::Launch,
                ..
            })
        ));
        let mut launch = script("exit 0");
        launch.cwd = missing.clone();
        assert!(matches!(
            Process::spawn(launch),
            Err(Failure::Io {
                stage: Stage::Launch,
                ..
            })
        ));
    }
    let failed = handles();
    for _ in 0..4 {
        let process = Process::spawn(script("Start-Sleep -Seconds 30")).unwrap();
        let pid = process.id();
        drop(process);
        reaped(pid);
    }
    let live = handles();
    let mut leaked = None;
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let process = Process::spawn(script("Start-Sleep -Seconds 30")).unwrap();
        leaked = Some(process.id());
        panic!("fixture cancellation");
    }));
    assert!(panic.is_err());
    reaped(leaked.unwrap());
    let after = handles();
    assert!(
        after <= baseline + 4,
        "handle count grew: baseline={baseline} failed={failed} live={live} after={after}"
    );
}
