use std::process::Command;

#[test]
fn capabilities() {
    let output = Command::new(env!("CARGO_BIN_EXE_bagua"))
        .args(["capabilities", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["runtime"], false);
    assert_eq!(value["components"].as_array().unwrap().len(), 7);
    assert_eq!(value["excluded"].as_array().unwrap().len(), 8);
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
    assert!(text.contains("models only"));
    assert!(text.contains("not implemented"));
}
