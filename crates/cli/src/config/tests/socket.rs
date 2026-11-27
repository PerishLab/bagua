use clap::Parser;
use std::{fs, path::Path};

#[test]
fn precedence() {
    let root = tempfile::tempdir().unwrap();
    let explicit = root.path().join("explicit.toml");
    let configdir = root.path().join("bagua");
    fs::create_dir_all(&configdir).unwrap();
    let discovered = configdir.join("bagua.toml");
    let pinned = if cfg!(windows) {
        r"C:\pinned.sock"
    } else {
        "/pinned.sock"
    };
    let ambient = if cfg!(windows) {
        r"C:\ambient.sock"
    } else {
        "/ambient.sock"
    };
    let envsocket = if cfg!(windows) {
        r"C:\env.sock"
    } else {
        "/env.sock"
    };
    let clisocket = if cfg!(windows) {
        r"C:\cli.sock"
    } else {
        "/cli.sock"
    };
    let expected = if cfg!(windows) {
        root.path().join("bagua").join("runtime").join("host.sock")
    } else {
        root.path().join("bagua/host.sock")
    };
    let home = root.path().to_string_lossy().into_owned();
    fs::write(&explicit, format!("[host]\nsocket='{pinned}'\n")).unwrap();
    fs::write(&discovered, format!("[host]\nsocket='{ambient}'\n")).unwrap();
    let resolve = |argv: &[&str], socket: Option<&str>| {
        let mut args = crate::args::Args::parse_from(argv);
        let loaded = super::super::load(&args, &|key| match key {
            "BAGUA_SOCKET" => socket.map(Into::into),
            "LOCALAPPDATA" | "XDG_CONFIG_HOME" | "XDG_RUNTIME_DIR" => Some(home.as_str().into()),
            _ => None,
        })
        .unwrap();
        loaded.apply(&mut args);
        (args, loaded)
    };
    let (args, loaded) = resolve(&["bagua", "--no-config", "list"], None);
    assert_eq!(args.socket.as_deref(), Some(expected.as_path()));
    assert_eq!(loaded.runtime(), Some(expected.as_path()));
    let cfg = explicit.to_str().unwrap();
    for (argv, env, want) in [
        (vec!["bagua", "list"], Some(""), Some(ambient)),
        (vec!["bagua", "list"], Some(envsocket), Some(envsocket)),
        (
            vec!["bagua", "--config", cfg, "list"],
            Some(envsocket),
            Some(pinned),
        ),
        (
            vec!["bagua", "--config", cfg, "list"],
            Some("relative.sock"),
            Some(pinned),
        ),
        (
            vec!["bagua", "--config", cfg, "--socket", clisocket, "list"],
            Some(envsocket),
            Some(clisocket),
        ),
        (
            vec!["bagua", "--config", cfg, "capabilities"],
            Some(envsocket),
            None,
        ),
    ] {
        let (args, loaded) = resolve(&argv, env);
        assert_eq!(args.socket.as_deref(), want.map(Path::new));
        assert!(loaded.runtime().is_none());
    }
    let bad = crate::args::Args::parse_from(["bagua", "--no-config", "list"]);
    assert!(
        super::super::load(&bad, &|key| (key == "BAGUA_SOCKET")
            .then(|| "relative.sock".into()))
        .is_err()
    );
    let (args, loaded) = resolve(&["bagua", "--no-config", "config"], Some(envsocket));
    assert_eq!(
        loaded.report(&args)["host"],
        serde_json::json!({"socket": envsocket, "default": false})
    );
}
