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
    let socket = if cfg!(windows) {
        r"C:\another.sock"
    } else {
        "/another.sock"
    };
    fs::write(
        &path,
        format!("[host]\nsocket='{socket}'\n[client]\nprefix='C-a'\n"),
    )
    .unwrap();
    assert!(config.reload().is_err());
    assert_eq!(config.prefix(), "C-q");
    fs::write(&path, "[history]\nlines=20000\n[client]\nprefix='C-a'\n").unwrap();
    assert!(config.reload().is_err());
    assert_eq!(config.history().lines, 50_000);
    assert_eq!(config.prefix(), "C-q");
    let (default, other) = if cfg!(windows) {
        ("launch", "foreground")
    } else {
        ("foreground", "launch")
    };
    fs::write(
        &path,
        format!("[workspace]\ncwd='{other}'\n[client]\nprefix='C-a'\n"),
    )
    .unwrap();
    assert!(config.reload().is_err());
    assert_eq!(config.workspace().cwd, default);
    assert_eq!(config.prefix(), "C-q");
    fs::write(&path, "[locus]\nenabled=true\n[client]\nprefix='C-a'\n").unwrap();
    assert!(config.reload().is_err());
    assert_eq!(config.prefix(), "C-q");
    assert!(config.locus().is_none());
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
    let (executable, cwd) = if cfg!(windows) {
        (r"C:\bin\sh", r"C:\")
    } else {
        ("/bin/sh", "/")
    };
    fs::write(
        &path,
        format!("[program]\nexecutable='{executable}'\nargs=[]\ncwd='{cwd}'\nenvironment={{}}\n"),
    )
    .unwrap();
    assert!(config.reload().is_err());
    assert_eq!(config.prefix(), "C-q");
}

#[cfg(unix)]
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

#[cfg(windows)]
#[test]
fn runtime() {
    let values = [
        ("LOCALAPPDATA", r"C:\Users\example\AppData\Local"),
        ("USERPROFILE", r"C:\Users\example"),
    ];
    for (start, expected) in [
        (0, r"C:\Users\example\AppData\Local\bagua\runtime\host.sock"),
        (1, r"C:\Users\example\AppData\Local\bagua\runtime\host.sock"),
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
    let (executable, cwd) = if cfg!(windows) {
        (r"C:\bin\sh", r"C:\")
    } else {
        ("/bin/sh", "/")
    };
    let mut program = protocol::wire::Program {
        executable: executable.into(),
        args: vec!["\u{1}".repeat(30_000)],
        cwd: cwd.into(),
        environment: Default::default(),
    };
    assert!(super::launch::check(&program).is_err());
    program.args = vec!["x".repeat(30_000)];
    assert!(super::launch::check(&program).is_ok());
}
#[test]
fn pointer() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("bagua.toml");
    fs::write(&path, "[client]\nprefix='C-q'\n").unwrap();
    let mut config = Loaded::read(Some(path.clone()), true).unwrap();
    for source in [
        "[client]\nprefix='C-a'\n[clipboard]\ntarget='shell'\n",
        "[clipboard]\nlimit=0\n",
        "[clipboard]\nlimit=1048577\n",
        "[mouse]\nunknown=true\n",
    ] {
        fs::write(&path, source).unwrap();
        assert!(config.reload().is_err());
        assert_eq!(config.prefix(), "C-q");
        assert_eq!(config.clipboard(), client::Clipboard::default());
        assert_eq!(config.mouse(), client::Mouse::default());
    }
    fs::write(&path, "[client]\nprefix='C-a'\n[mouse]\nenabled=false\nresize=false\ndrag_select=false\ncopy_on_release=false\n[clipboard]\ntarget='osc52'\nlimit=8192\n").unwrap();
    config.reload().unwrap();
    assert_eq!(config.prefix(), "C-a");
    assert_eq!(
        config.clipboard(),
        client::Clipboard::new("osc52", 8192).unwrap()
    );
    assert_eq!(
        config.mouse(),
        client::Mouse {
            enabled: false,
            resize: false,
            drag: false,
            release: false
        }
    );
}
#[test]
fn decor() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("bagua.toml");
    fs::write(&path, "[client]\nprefix='C-q'\n").unwrap();
    let mut config = Loaded::read(Some(path.clone()), true).unwrap();
    let before = config.decor().clone();
    for source in [
        "[client]\nprefix='C-a'\n[header]\ncomponents=['shell']\n",
        "[header]\nactive=256\n",
        "[status]\ntimezone='local'\n",
        "[status]\nrefresh_seconds=0\n",
        "[status]\ncomponents=['clock','clock']\n",
    ] {
        fs::write(&path, source).unwrap();
        assert!(config.reload().is_err());
        assert_eq!(config.prefix(), "C-q");
        assert_eq!(config.decor(), &before);
    }
    fs::write(&path,"[header]\nenabled=true\ncomponents=['command','cwd']\nactive=12\ninactive=0\ncolor=false\n[status]\nseparator=true\ntimezone='Asia/Shanghai'\nrefresh_seconds=1\n").unwrap();
    config.reload().unwrap();
    assert!(config.decor().header.enabled);
    assert!(!config.decor().header.color);
    assert_eq!(config.decor().header.active, 12);
    assert!(config.decor().status.separator);
    assert_eq!(config.decor().status.refresh(), Duration::from_secs(1));
}

#[test]
fn templates() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("bagua.toml");
    fs::write(&path,"[header]\nformat='W{{window.index}} P{{pane.index}}'\nposition='bottom'\n[status]\nposition='top'\n[[status.lines]]\nleft='{{session.name}}'\nright='{{clock.time}}'\n").unwrap();
    let mut config = Loaded::read(Some(path.clone()), true).unwrap();
    assert!(config.decor().header.template.is_some());
    assert_eq!(config.decor().header.edge, workspace::Edge::Bottom);
    assert_eq!(config.decor().status.edge, workspace::Edge::Top);
    assert_eq!(config.decor().status.rows(), 1);
    let before = config.decor().clone();
    for text in [
        "[header]\nformat='{{unknown}}'\n",
        "[header]\ncomponents=['pane']\nformat='text'\n",
        "[status]\ncomponents=['clock']\nlines=[]\n",
        "[status]\nlines=[]\n",
        "[status]\nposition='left'\n",
        "[[status.lines]]\nleft='{{#if pane.active}}unterminated'\n",
    ] {
        fs::write(&path, text).unwrap();
        assert!(config.reload().is_err());
        assert_eq!(config.decor(), &before);
    }
    fs::write(
        &path,
        "[header]\ncomponents=['pane']\n[status]\ncomponents=['clock']\nseparator=true\n",
    )
    .unwrap();
    config.reload().unwrap();
    assert!(config.decor().header.template.is_none());
    assert!(config.decor().status.layout.is_none());
    assert_eq!(config.decor().status.rows(), 2);
}

mod socket;
