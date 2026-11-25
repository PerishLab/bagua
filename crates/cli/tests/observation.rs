use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

const BUFFER: &str = if cfg!(windows) {
    r"bagua\state\locus"
} else {
    ".local/state/bagua/locus"
};

fn run(home: &Path, extra: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_bagua"));
    command.env_clear();
    if cfg!(windows) {
        command
            .env("LOCALAPPDATA", home)
            .env("USERPROFILE", home)
            .env(
                "SYSTEMROOT",
                std::env::var_os("SYSTEMROOT").unwrap_or_else(|| r"C:\Windows".into()),
            );
    } else {
        command.env("HOME", home).env("PATH", "/usr/bin:/bin");
    }
    for (key, value) in extra {
        command.env(key, value);
    }
    command
        .args(["--config"])
        .arg(home.join("bagua.toml"))
        .args(["--json", "capabilities"])
        .output()
        .unwrap()
}

fn configure(home: &Path, source: &str) {
    fs::write(home.join("bagua.toml"), source).unwrap();
}

fn baseline(home: &Path) -> Output {
    configure(home, "");
    let output = run(home, &[]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    output
}

#[test]
fn disabled() {
    let home = tempfile::tempdir().unwrap();
    let before = baseline(home.path());
    configure(
        home.path(),
        "[locus]\nenabled=false\nendpoint='INVALID-ENDPOINT-CANARY'\n",
    );
    let after = run(
        home.path(),
        &[
            ("LOCUS_PARENT", "INVALID\nPARENT"),
            ("LOCUS_API", "INVALID-ENDPOINT"),
        ],
    );
    assert_eq!(after.status.code(), before.status.code());
    assert_eq!(after.stdout, before.stdout);
    assert!(after.stderr.is_empty());
    assert!(!home.path().join(BUFFER).exists());
}

#[cfg(any(
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "windows", target_arch = "x86_64")
))]
#[test]
fn invalid() {
    let home = tempfile::tempdir().unwrap();
    let before = baseline(home.path());
    for source in [
        "[locus]\nenabled='PRIVATE-CONFIG-CANARY'\n",
        "[locus]\nenabled=true\nendpoint=42\n",
        "[locus]\nenabled=true\nunknown='PRIVATE-CONFIG-CANARY'\n",
    ] {
        configure(home.path(), source);
        let after = run(home.path(), &[]);
        assert_eq!(after.status.code(), before.status.code());
        assert_eq!(after.stdout, before.stdout);
        let diagnostic: serde_json::Value = serde_json::from_slice(&after.stderr).unwrap();
        assert_eq!(diagnostic["diagnostic"]["code"], "config");
        assert!(
            !String::from_utf8(after.stderr)
                .unwrap()
                .contains("PRIVATE-CONFIG-CANARY")
        );
        assert!(!home.path().join(BUFFER).exists());
    }
}

#[cfg(any(
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "windows", target_arch = "x86_64")
))]
#[test]
fn parent() {
    let home = tempfile::tempdir().unwrap();
    let before = baseline(home.path());
    configure(home.path(), "[locus]\nenabled=true\n");
    for parent in ["", "PRIVATE-PARENT-CANARY\n", &"x".repeat(513)] {
        let after = run(home.path(), &[("LOCUS_PARENT", parent)]);
        assert_eq!(after.status.code(), before.status.code());
        assert_eq!(after.stdout, before.stdout);
        let diagnostic: serde_json::Value = serde_json::from_slice(&after.stderr).unwrap();
        assert_eq!(diagnostic["diagnostic"]["code"], "parent");
        assert!(
            !String::from_utf8(after.stderr)
                .unwrap()
                .contains("PRIVATE-PARENT-CANARY")
        );
        assert!(!home.path().join(BUFFER).exists());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn private() {
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    let before = baseline(home.path());
    let path = home.path().join(BUFFER);
    fs::create_dir_all(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    configure(home.path(), "[locus]\nenabled=true\n");
    let after = run(home.path(), &[]);
    assert_eq!(after.status.code(), before.status.code());
    assert_eq!(after.stdout, before.stdout);
    let diagnostic: serde_json::Value = serde_json::from_slice(&after.stderr).unwrap();
    assert_eq!(diagnostic["diagnostic"]["code"], "state");
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o755
    );
    assert_eq!(fs::read_dir(path).unwrap().count(), 0);
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
#[test]
fn private() {
    let home = tempfile::tempdir().unwrap();
    let before = baseline(home.path());
    let path = home.path().join(BUFFER);
    fs::create_dir_all(&path).unwrap();
    assert!(!bagua::ipc::private(&path).unwrap());
    configure(home.path(), "[locus]\nenabled=true\n");
    let after = run(home.path(), &[]);
    assert_eq!(after.status.code(), before.status.code());
    assert_eq!(after.stdout, before.stdout);
    let diagnostic: serde_json::Value = serde_json::from_slice(&after.stderr).unwrap();
    assert_eq!(diagnostic["diagnostic"]["code"], "state");
    assert!(!bagua::ipc::private(&path).unwrap());
    assert_eq!(fs::read_dir(path).unwrap().count(), 0);
}
