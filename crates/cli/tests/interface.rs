use std::process::Command;

#[test]
fn capabilities() {
    let output = Command::new(env!("CARGO_BIN_EXE_bagua"))
        .arg("--no-config")
        .args(["capabilities", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["runtime"], bagua::profile().runtime);
    assert_eq!(value["request"], 1);
    assert_eq!(value["components"].as_array().unwrap().len(), 7);
    assert_eq!(value["excluded"].as_array().unwrap().len(), 8);
}

#[test]
fn routing() {
    let commands = [
        vec!["watch", "pane", "3"],
        vec!["unwatch", "pane", "3"],
        vec!["split", "3", "--axis", "columns"],
        vec!["acquire", "1", "--fixed", "--columns", "81", "--rows", "25"],
        vec!["focus", "3"],
        vec!["close", "pane", "3"],
        vec!["resize", "1", "--columns", "40", "--rows", "10"],
        vec!["size", "1", "--fixed"],
        vec!["send", "3", "text"],
        vec!["attach", "1"],
    ];
    for args in commands {
        let output = Command::new(env!("CARGO_BIN_EXE_bagua"))
            .env_clear()
            .arg("--no-config")
            .args(args)
            .args(["--json", "--request", "17"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        let value: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(value["error"], "unavailable");
        assert_eq!(value["request"], 17);
    }
}

#[test]
fn refusal() {
    let commands = [
        (vec!["attach", "0"], "identity"),
        (
            vec!["split", "3", "--axis", "rows", "--ratio", "1000"],
            "geometry",
        ),
        (vec!["new", "--columns", "0"], "geometry"),
        (vec!["capabilities", "--request", "0"], "identity"),
    ];
    for (args, error) in commands {
        let output = Command::new(env!("CARGO_BIN_EXE_bagua"))
            .arg("--no-config")
            .args(args)
            .arg("--json")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let value: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(value["error"], error);
    }
}

#[test]
fn unavailable() {
    let output = Command::new(env!("CARGO_BIN_EXE_bagua"))
        .env_clear()
        .arg("--no-config")
        .args(["new", "--json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"], "unavailable");
}

#[test]
fn discovery() {
    let home = tempfile::tempdir().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_bagua"));
    command.env_clear();
    if cfg!(windows) {
        command
            .env("LOCALAPPDATA", home.path())
            .env("USERPROFILE", home.path())
            .env(
                "SYSTEMROOT",
                std::env::var_os("SYSTEMROOT").unwrap_or_else(|| r"C:\Windows".into()),
            );
    } else {
        command
            .env("HOME", home.path())
            .env("XDG_RUNTIME_DIR", home.path());
    }
    let output = command
        .args(["--no-config", "--json", "list"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"], "io");

    let mut invalid = Command::new(env!("CARGO_BIN_EXE_bagua"));
    invalid.env_clear();
    if cfg!(windows) {
        invalid.env(
            "SYSTEMROOT",
            std::env::var_os("SYSTEMROOT").unwrap_or_else(|| r"C:\Windows".into()),
        );
    }
    let output = invalid
        .env("BAGUA_SOCKET", "relative.sock")
        .args(["--no-config", "--json", "list"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"], "configuration");
}

#[test]
fn help() {
    let output = Command::new(env!("CARGO_BIN_EXE_bagua"))
        .arg("--no-config")
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("machine control"));
    assert!(text.contains("interactive"));
}

#[test]
fn conflict() {
    let output = Command::new(env!("CARGO_BIN_EXE_bagua"))
        .arg("--no-config")
        .args(["inspect", "1", "--text", "--json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}
