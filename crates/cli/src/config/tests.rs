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
