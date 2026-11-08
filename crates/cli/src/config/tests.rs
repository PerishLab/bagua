use super::Loaded;
use std::{fs, time::Duration};

#[test]
fn transaction() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("bagua.toml");
    fs::write(&path, "[client]\nprefix='C-q'\nescape_ms=20\n").unwrap();
    let mut config = Loaded::read(Some(path.clone()), true).unwrap();
    fs::write(&path, "[client]\nprefix='C-a'\nescape_ms=1001\n").unwrap();
    assert!(config.reload().is_err());
    assert_eq!(config.prefix(), "C-q");
    assert_eq!(config.delay(), Duration::from_millis(20));
    fs::write(
        &path,
        "[host]\nsocket='/another.sock'\n[client]\nprefix='C-a'\n",
    )
    .unwrap();
    assert!(config.reload().is_err());
    assert_eq!(config.prefix(), "C-q");
    fs::write(&path, "[history]\nlines=20000\n[client]\nprefix='C-a'\n").unwrap();
    assert!(config.reload().is_err());
    assert_eq!(config.history().lines, 50_000);
    assert_eq!(config.prefix(), "C-q");
    fs::write(&path, "[workspace]\ncwd='launch'\n[client]\nprefix='C-a'\n").unwrap();
    assert!(config.reload().is_err());
    assert_eq!(config.workspace().cwd, "foreground");
    assert_eq!(config.prefix(), "C-q");
    fs::write(&path, "[client]\nprefix='C-a'\nescape_ms=0\n").unwrap();
    config.reload().unwrap();
    assert_eq!(config.prefix(), "C-a");
    assert_eq!(config.delay(), Duration::ZERO);
    fs::remove_file(&path).unwrap();
    assert!(config.reload().is_err());
    assert_eq!(config.prefix(), "C-a");
}

#[test]
fn startup() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("bagua.toml");
    let mut config = Loaded::read(Some(path.clone()), false).unwrap();
    assert!(!config.present);
    assert!(config.reload().is_err());
    fs::write(&path, "[client]\nprefix='C-q'\n").unwrap();
    config.reload().unwrap();
    assert!(config.present);
    assert!(!config.explicit);
    assert_eq!(config.prefix(), "C-q");
    fs::write(
        &path,
        "[program]\nexecutable='/bin/sh'\nargs=[]\ncwd='/'\nenvironment={}\n",
    )
    .unwrap();
    assert!(config.reload().is_err());
    assert_eq!(config.prefix(), "C-q");
}

#[test]
fn runtime() {
    let values = [
        ("XDG_RUNTIME_DIR", "/runtime"),
        ("XDG_STATE_HOME", "/state"),
        ("HOME", "/home/example"),
    ];
    for (start, expected) in [
        (0, "/runtime/bagua/host.sock"),
        (1, "/state/bagua/host.sock"),
        (2, "/home/example/.local/state/bagua/host.sock"),
    ] {
        let path = super::runtime::path(&|key| {
            values[start..]
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| (*value).into())
        })
        .unwrap()
        .unwrap();
        assert_eq!(path, std::path::Path::new(expected));
    }
    assert!(super::runtime::path(&|_| Some("relative".into())).is_err());
}

#[test]
fn encoding() {
    let mut program = protocol::wire::Program {
        executable: "/bin/sh".into(),
        args: vec!["\u{1}".repeat(30_000)],
        cwd: "/".into(),
        environment: Default::default(),
    };
    assert!(super::launch::check(&program).is_err());
    program.args = vec!["x".repeat(30_000)];
    assert!(super::launch::check(&program).is_ok());
}
