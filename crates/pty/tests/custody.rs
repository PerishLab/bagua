#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

mod support;

use pty::{Failure, Process, Stage};
use rustix::{
    io::Errno,
    process::{Pid, Signal, WaitOptions, kill_process, waitpid},
};

fn descriptors() -> usize {
    std::fs::read_dir("/proc/self/fd").unwrap().count()
}

fn reaped(id: u32) {
    let pid = Pid::from_raw(i32::try_from(id).unwrap()).unwrap();
    match waitpid(Some(pid), WaitOptions::NOHANG) {
        Err(Errno::CHILD) => return,
        Ok(None) => {
            kill_process(pid, Signal::KILL).unwrap();
            waitpid(Some(pid), WaitOptions::empty()).unwrap();
        }
        other => assert!(other.is_ok(), "unexpected wait result: {other:?}"),
    }
    panic!("PTY owner did not reap its controlled child");
}

#[test]
fn custody() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("absent");
    let before = descriptors();
    for _ in 0..16 {
        let mut launch = support::shell("exit 0");
        launch.program = missing.clone();
        assert!(matches!(
            Process::spawn(launch),
            Err(Failure::Io {
                stage: Stage::Launch,
                ..
            })
        ));
        assert_eq!(descriptors(), before);
        let mut launch = support::shell("exit 0");
        launch.cwd = missing.clone();
        assert!(matches!(
            Process::spawn(launch),
            Err(Failure::Io {
                stage: Stage::Launch,
                ..
            })
        ));
        assert_eq!(descriptors(), before);
    }
    for _ in 0..8 {
        let process = Process::spawn(support::shell("exec /bin/sleep 60")).unwrap();
        let id = process.id();
        assert!(descriptors() > before);
        drop(process);
        reaped(id);
        assert_eq!(descriptors(), before);
    }
    let mut id = None;
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let process = Process::spawn(support::shell("exec /bin/sleep 60")).unwrap();
        id = Some(process.id());
        panic!("fixture cancellation");
    }));
    assert!(panic.is_err());
    reaped(id.unwrap());
    assert_eq!(descriptors(), before);
}
