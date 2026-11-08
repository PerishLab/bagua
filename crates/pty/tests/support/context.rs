use crate::{runner::Runner, support};
use std::time::{Duration, Instant};

#[test]
fn directory() {
    let root = tempfile::tempdir().unwrap();
    let nested = root.path().join("nested");
    std::fs::create_dir(&nested).unwrap();
    let mut launch = support::shell("cd nested; printf READY; read -r token");
    launch.cwd = root.path().into();
    let mut runner = Runner::new(launch);
    runner.until(b"READY");
    let context = runner.process.context().unwrap();
    assert_eq!(context.cwd, nested);
    assert_eq!(context.pid, runner.process.id());
    assert_eq!(context.session, runner.process.id());
    assert!(context.started > 0);
    assert!(!context.command.is_empty());
    runner.process.close().unwrap();
    assert!(runner.process.context().is_err());
}

#[test]
fn foreground() {
    let root = tempfile::tempdir().unwrap();
    let nested = root.path().join("nested");
    std::fs::create_dir(&nested).unwrap();
    let mut launch = support::shell("");
    launch.args = vec!["-i".into()];
    launch.cwd = root.path().into();
    launch.environment = radicula_process::Environment::new([
        ("PATH".into(), "/usr/bin:/bin".into()),
        ("TERM".into(), "dumb".into()),
        ("PS1".into(), "PROMPT>".into()),
    ]);
    let mut runner = Runner::new(launch);
    runner.until(b"PROMPT>");
    runner.send(b"cd nested\nsleep 60\n");
    let deadline = Instant::now() + Duration::from_secs(5);
    let context = loop {
        if let Ok(context) = runner.process.context()
            && context.pid != runner.process.id()
            && context.command == "sleep"
        {
            break context;
        }
        assert!(Instant::now() < deadline, "foreground group did not change");
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(context.cwd, nested);
    assert_eq!(context.session, runner.process.id());
    runner.send(&[3]);
    runner.process.close().unwrap();
}

#[test]
fn removed() {
    let root = tempfile::tempdir().unwrap();
    let nested = root.path().join("nested");
    std::fs::create_dir(&nested).unwrap();
    let mut launch = support::shell("rmdir ../nested; printf READY; read -r token");
    launch.cwd = nested;
    let mut runner = Runner::new(launch);
    runner.until(b"READY");
    assert!(runner.process.context().is_err());
    runner.process.close().unwrap();
}

#[test]
fn permission() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let nested = root.path().join("nested");
    std::fs::create_dir(&nested).unwrap();
    let mut launch = support::shell("printf READY; read -r token");
    launch.cwd = nested.clone();
    let mut runner = Runner::new(launch);
    runner.until(b"READY");
    std::fs::set_permissions(&nested, std::fs::Permissions::from_mode(0o000)).unwrap();
    let denied = runner.process.context().is_err();
    std::fs::set_permissions(&nested, std::fs::Permissions::from_mode(0o700)).unwrap();
    if rustix::process::geteuid() != rustix::process::Uid::ROOT {
        assert!(denied);
    }
    assert_eq!(runner.process.context().unwrap().cwd, nested);
    runner.process.close().unwrap();
}
