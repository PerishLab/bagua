use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn run(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bagua"))
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", "")
        .current_dir(home)
        .args(args)
        .output()
        .unwrap()
}

fn read(output: &Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn discovery() {
    let home = tempfile::tempdir().unwrap();
    fs::write(home.path().join("bagua.toml"), "invalid = true").unwrap();
    let config = read(&run(home.path(), &["config"]));
    assert_eq!(config["client"]["prefix"], "C-g");
    assert_eq!(config["loaded"], false);
    let directory = home.path().join(".config/bagua");
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("bagua.toml"), "[client]\nprefix='C-q'\n").unwrap();
    assert_eq!(
        read(&run(home.path(), &["config"]))["client"]["prefix"],
        "C-q"
    );
    assert_eq!(
        read(&run(home.path(), &["--no-config", "config"]))["client"]["prefix"],
        "C-g"
    );
}

#[test]
fn priority() {
    let home = tempfile::tempdir().unwrap();
    let directory = home.path().join("xdg/bagua");
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("bagua.toml"), "[client]\nprefix='C-q'\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_bagua"))
        .env_clear()
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", directory.parent().unwrap())
        .args(["config"])
        .output()
        .unwrap();
    assert_eq!(read(&output)["client"]["prefix"], "C-q");
    let path = home.path().join("chosen.toml");
    fs::write(&path, "[client]\nprefix='C-a'\n").unwrap();
    let config = read(&run(
        home.path(),
        &["--config", path.to_str().unwrap(), "config"],
    ));
    assert_eq!(config["client"]["prefix"], "C-a");
    assert_eq!(config["explicit"], true);
    assert_eq!(config["loaded"], true);
}

#[test]
fn diagnosis() {
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join("chosen.toml");
    let cases = [
        ("version=2", "version"),
        ("unknown=true", "unknown field"),
        ("[history]\nrows=50000", "unknown field"),
        ("[history]\nlines=50001", "history"),
        ("[history]\nbytes=67108865", "history"),
        ("[history]\nbytes=1024\ntotal=1023", "history"),
        ("[history]\ntotal=268435457", "history"),
        ("[client]\nprefix='C-h'", "client.prefix"),
        ("[client]\nescape_ms=1001", "client.escape_ms"),
        ("[client]\nprefix='C-q'\nprefix='C-a'", "duplicate"),
        ("[client]\nextra=true", "unknown field"),
        ("[host]\nsocket='relative.sock'", "host.socket"),
        ("[workspace]\nname=' bad '", "workspace.name"),
        ("[workspace]\ncwd='infer'", "workspace.cwd"),
        ("[workspace]\nunknown=true", "unknown field"),
        ("[bindings.direct]\n'Ctrl-g'='detach'", "bindings"),
        ("[bindings.direct]\n'Alt-q'='unknown'", "bindings"),
        ("[bindings]\nunknown=true", "unknown field"),
    ];
    for (text, marker) in cases {
        fs::write(&path, text).unwrap();
        let output = run(
            home.path(),
            &["--config", path.to_str().unwrap(), "config", "--json"],
        );
        assert_eq!(output.status.code(), Some(2));
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"], "configuration");
        assert!(
            error["message"].as_str().unwrap().contains(marker),
            "{error}"
        );
        assert!(
            error["message"]
                .as_str()
                .unwrap()
                .contains(path.to_str().unwrap())
        );
    }
}

#[test]
fn private() {
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join("chosen.toml");
    fs::write(&path, "version=1\n[host]\nsocket='/file.sock'\n[program]\nexecutable='/bin/sh'\nargs=['-i']\ncwd='/'\n[program.environment]\nPRIVATE='not-for-diagnostics'\n").unwrap();
    let output = run(
        home.path(),
        &[
            "--config",
            path.to_str().unwrap(),
            "--socket",
            "/cli.sock",
            "config",
        ],
    );
    let config = read(&output);
    assert_eq!(config["host"]["socket"], "/cli.sock");
    assert_eq!(
        config["program"]["environment"],
        serde_json::json!(["PRIVATE"])
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("not-for-diagnostics"));
}

#[test]
fn absent() {
    let home = tempfile::tempdir().unwrap();
    let output = run(
        home.path(),
        &["--config", "absent.toml", "config", "--json"],
    );
    assert_eq!(output.status.code(), Some(2));
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"], "configuration");
    assert!(error["message"].as_str().unwrap().contains("absent.toml"));
}
