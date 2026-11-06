use std::process::Command;

#[test]
fn capabilities() {
    let output = Command::new(env!("CARGO_BIN_EXE_bagua"))
        .args(["capabilities", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value["runtime"],
        cfg!(all(target_os = "linux", target_arch = "x86_64"))
    );
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
        .args(["new", "--json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"], "unavailable");
}

#[test]
fn help() {
    let output = Command::new(env!("CARGO_BIN_EXE_bagua"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("machine control"));
    assert!(text.contains("interactive"));
}
