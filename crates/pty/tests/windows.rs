#![cfg(all(target_os = "windows", target_arch = "x86_64"))]

#[path = "support/runner.rs"]
mod runner;

use pty::{CHUNK, Failure, Interest, Launch, Process, Size, Transfer};
use radicula_process::Environment;
use runner::Runner;
use std::{
    ffi::OsString,
    path::PathBuf,
    time::{Duration, Instant},
};

fn powershell() -> PathBuf {
    let root = std::env::var_os("SystemRoot")
        .or_else(|| std::env::var_os("WINDIR"))
        .unwrap_or_else(|| OsString::from(r"C:\Windows"));
    PathBuf::from(root)
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe")
}

fn environment(extra: &[(&str, &str)]) -> Environment {
    let mut pairs: Vec<(OsString, OsString)> = Vec::new();
    for key in [
        "SYSTEMROOT",
        "WINDIR",
        "COMSPEC",
        "PATH",
        "PATHEXT",
        "TEMP",
        "TMP",
    ] {
        if let Some(value) = std::env::var_os(key) {
            pairs.push((OsString::from(key), value));
        }
    }
    for (key, value) in extra {
        pairs.push((OsString::from(key), OsString::from(value)));
    }
    Environment::new(pairs)
}

fn script(body: &str) -> Launch {
    let cwd = std::env::temp_dir();
    let mut launch = Launch::new(
        powershell(),
        cwd,
        environment(&[]),
        Size::new(80, 24).unwrap(),
    );
    launch.args = vec![
        "-NoLogo".into(),
        "-NoProfile".into(),
        "-NonInteractive".into(),
        "-Command".into(),
        body.into(),
    ];
    launch
}

#[test]
fn terminal() {
    let mut runner = Runner::new(script("[Console]::Write('tty-ok'); exit 0"));
    let exit = runner.exit();
    assert_eq!(exit.code, Some(0));
    assert_eq!(exit.signal, None);
    let text = String::from_utf8_lossy(&runner.output);
    assert!(text.contains("tty-ok"), "unexpected output: {text:?}");
}

#[test]
fn explicit() {
    let directory = tempfile::tempdir().unwrap();
    let mut launch = Launch::new(
        powershell(),
        directory.path().into(),
        environment(&[("BAGUA_FIXTURE", "present")]),
        Size::new(80, 24).unwrap(),
    );
    launch.args = vec![
        "-NoLogo".into(),
        "-NoProfile".into(),
        "-NonInteractive".into(),
        "-Command".into(),
        "[Console]::Write($env:BAGUA_FIXTURE + '|' + (Get-Location).Path); exit 0".into(),
    ];
    let mut runner = Runner::new(launch);
    assert!(runner.exit().success());
    let text = String::from_utf8_lossy(&runner.output);
    assert!(text.contains("present|"), "unexpected output: {text:?}");
    let leaf = directory.path().file_name().unwrap().to_str().unwrap();
    assert!(text.contains(leaf), "cwd missing from output: {text:?}");
}

#[test]
fn resize() {
    let mut runner = Runner::new(script(
        "[Console]::Write('READY'); $null = [Console]::ReadLine(); [Console]::Write([string][Console]::WindowWidth + 'x' + [string][Console]::WindowHeight); exit 0",
    ));
    runner.until(b"READY");
    assert_eq!(runner.process.size().unwrap(), Size::new(80, 24).unwrap());
    runner.process.resize(Size::new(100, 30).unwrap()).unwrap();
    assert_eq!(runner.process.size().unwrap(), Size::new(100, 30).unwrap());
    std::thread::sleep(Duration::from_millis(50));
    runner.send(b"go\r\n");
    assert!(runner.exit().success());
    let text = String::from_utf8_lossy(&runner.output);
    assert!(text.contains("100x30"), "resize not observed: {text:?}");
}

#[test]
fn flood() {
    let mut runner = Runner::new(script("[Console]::Write('A' * 65536 + 'END'); exit 0"));
    assert!(runner.exit().success());
    let text = String::from_utf8_lossy(&runner.output);
    assert!(text.contains("END"), "flood truncated");
    assert!(runner.output.len() >= 65_536);
}

#[test]
fn context() {
    let mut process = Process::spawn(script("Start-Sleep -Seconds 30")).unwrap();
    assert!(matches!(process.context(), Err(Failure::Unsupported)));
    process.close().unwrap();
}

#[test]
fn validation() {
    assert!(Size::new(0, 24).is_none());
    assert!(Size::new(80, 0).is_none());
    let mut launch = script("exit 0");
    launch.program = "powershell.exe".into();
    assert!(matches!(Process::spawn(launch), Err(Failure::Input(_))));
    let mut launch = script("exit 0");
    launch.args = vec!["bad\0argument".into()];
    assert!(matches!(Process::spawn(launch), Err(Failure::Input(_))));
    let mut launch = script("exit 0");
    launch.environment = Environment::new([("bad=key".into(), "value".into())]);
    assert!(matches!(Process::spawn(launch), Err(Failure::Input(_))));
    let mut launch = script("exit 0");
    launch.args = vec!["x".repeat(65_537).into()];
    assert!(matches!(Process::spawn(launch), Err(Failure::Input(_))));

    let mut process = Process::spawn(script("Start-Sleep -Seconds 30")).unwrap();
    assert!(matches!(process.read(&mut []), Err(Failure::Input(_))));
    assert!(matches!(
        process.poll(Interest::Read, Duration::from_secs(2)),
        Err(Failure::Input(_))
    ));
    process.close().unwrap();
    assert!(process.status().closed());
    assert!(matches!(process.write(b"x"), Err(Failure::Closed)));
    assert!(matches!(
        process.resize(Size::new(80, 24).unwrap()),
        Err(Failure::Closed)
    ));
}

#[test]
fn nonblocking() {
    let mut process =
        Process::spawn(script("[Console]::Write('READY'); Start-Sleep -Seconds 30")).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut output = Vec::new();
    let mut chunk = [0u8; CHUNK];
    while !output.windows(5).any(|slice| slice == b"READY") {
        assert!(Instant::now() < deadline);
        match process.read(&mut chunk).unwrap() {
            Transfer::Bytes(count) => output.extend_from_slice(&chunk[..count]),
            Transfer::Pending => {
                process
                    .poll(Interest::Read, Duration::from_millis(10))
                    .unwrap();
            }
            Transfer::Eof => panic!("unexpected EOF"),
        }
    }
    let ready = process
        .poll(Interest::Write, Duration::from_millis(10))
        .unwrap();
    assert!(ready.write);
    process.close().unwrap();
}
