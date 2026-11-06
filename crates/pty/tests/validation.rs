#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

mod support;

use pty::{Failure, Interest, Process, Size};
use radicula_process::Environment;
use std::time::Duration;

#[test]
fn launch() {
    assert!(Size::new(0, 24).is_none());
    assert!(Size::new(80, 0).is_none());
    let mut launch = support::shell("exit 0");
    launch.program = "sh".into();
    assert!(matches!(Process::spawn(launch), Err(Failure::Input(_))));
    let mut launch = support::shell("exit 0");
    launch.args = vec!["bad\0argument".into()];
    assert!(matches!(Process::spawn(launch), Err(Failure::Input(_))));
    let mut launch = support::shell("exit 0");
    launch.environment = Environment::new([("bad=key".into(), "value".into())]);
    assert!(matches!(Process::spawn(launch), Err(Failure::Input(_))));
    let mut launch = support::shell("exit 0");
    launch.args = vec!["x".repeat(65_537).into()];
    assert!(matches!(Process::spawn(launch), Err(Failure::Input(_))));
}

#[test]
fn requests() {
    let mut process = Process::spawn(support::shell("exec /bin/sleep 60")).unwrap();
    assert!(matches!(process.read(&mut []), Err(Failure::Input(_))));
    assert!(matches!(
        process.poll(Interest::Read, Duration::from_secs(2)),
        Err(Failure::Input(_))
    ));
    process.close().unwrap();
    assert!(process.status().closed());
    assert!(matches!(process.write(b"x"), Err(Failure::Closed)));
    assert!(matches!(
        process.resize(Size::new(80, 24).unwrap()),
        Err(Failure::Closed)
    ));
}
