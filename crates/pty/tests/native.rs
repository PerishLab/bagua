#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

#[path = "support/runner.rs"]
mod runner;
mod support;

use pty::{CHUNK, Failure, Interest, Launch, Size, Transfer};
use radicula_process::Environment;
use runner::Runner;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[test]
fn terminal() {
    let mut runner = Runner::new(support::shell(
        "test -t 0 && test -t 1 && test -t 2 && : <>/dev/tty && printf tty-ok",
    ));
    assert!(runner.exit().success());
    assert_eq!(runner.output, b"tty-ok");
}

#[test]
fn explicit() {
    let directory = tempfile::tempdir().unwrap();
    let launch = Launch::new(
        PathBuf::from("/usr/bin/env"),
        directory.path().into(),
        Environment::new([("BAGUA_FIXTURE".into(), "present".into())]),
        Size::new(80, 24).unwrap(),
    );
    let mut runner = Runner::new(launch);
    assert!(runner.exit().success());
    assert_eq!(runner.output, b"BAGUA_FIXTURE=present\r\n");
    let launch = Launch::new(
        PathBuf::from("/bin/pwd"),
        directory.path().into(),
        Environment::new([]),
        Size::new(80, 24).unwrap(),
    );
    let mut runner = Runner::new(launch);
    assert!(runner.exit().success());
    assert_eq!(
        runner.output,
        format!("{}\r\n", directory.path().display()).as_bytes()
    );
    let mut launch = Launch::new(
        PathBuf::from("/usr/bin/printf"),
        directory.path().into(),
        Environment::new([]),
        Size::new(80, 24).unwrap(),
    );
    launch.args = vec!["%s".into(), "a b;$(false)".into()];
    let mut runner = Runner::new(launch);
    assert!(runner.exit().success());
    assert_eq!(runner.output, b"a b;$(false)");
}

#[test]
fn raw() {
    let mut runner = Runner::new(support::shell(
        "stty raw -echo; printf READY; dd bs=1 count=4 2>/dev/null",
    ));
    runner.until(b"READY");
    let bytes = [0, 255, 27, 127];
    runner.send(&bytes);
    assert!(runner.exit().success());
    assert_eq!(&runner.output[5..], bytes);
}

#[test]
fn resize() {
    let mut runner = Runner::new(support::shell(
        "trap 'stty size; printf WINCH' WINCH; printf READY; read -r value",
    ));
    runner.until(b"READY");
    assert_eq!(runner.process.size().unwrap(), Size::new(80, 24).unwrap());
    let size = Size::new(100, 40).unwrap();
    runner.process.resize(size).unwrap();
    assert_eq!(runner.process.size().unwrap(), size);
    runner.until(b"WINCH");
    assert!(runner.output.windows(8).any(|bytes| bytes == b"40 100\r\n"));
    runner.process.close().unwrap();
}

#[test]
fn flood() {
    let mut runner = Runner::new(support::shell(
        "stty raw -echo; printf READY; head -c 1048576 /dev/zero",
    ));
    runner.until(b"READY");
    std::thread::sleep(Duration::from_millis(20));
    assert_eq!(runner.process.observe().unwrap(), None);
    assert!(runner.exit().success());
    assert_eq!(runner.output.len(), 5 + 1_048_576);
    assert!(runner.output[5..].iter().all(|byte| *byte == 0));
}

#[test]
fn backpressure() {
    let mut runner = Runner::new(support::shell(
        "stty raw -echo; printf READY; exec /bin/sleep 60",
    ));
    runner.until(b"READY");
    let mut blocked = false;
    for _ in 0..1024 {
        match runner.process.write(&[b'x'; CHUNK]).unwrap() {
            Transfer::Bytes(count) => assert!(count > 0 && count <= CHUNK),
            Transfer::Pending => {
                blocked = true;
                break;
            }
            Transfer::Eof => panic!("fixture exited unexpectedly"),
        }
    }
    assert!(blocked);
    assert_eq!(runner.process.observe().unwrap(), None);
    runner.process.retire().unwrap();
    assert!(!runner.process.status().io);
    runner.process.close().unwrap();
    assert!(runner.process.status().closed());
}

#[test]
fn outcomes() {
    let mut runner = Runner::new(support::shell("exit 7"));
    let exit = runner.exit();
    assert_eq!(exit.code, Some(7));
    assert_eq!(exit.signal, None);
    let mut runner = Runner::new(support::shell("kill -TERM $$"));
    let exit = runner.exit();
    assert_eq!(exit.code, None);
    assert_eq!(exit.signal, Some(15));
}

#[test]
fn eof() {
    let mut runner = Runner::new(support::shell(
        "trap '' HUP; printf EOF; exec 0<&- 1>&- 2>&-; exec /bin/sleep 60",
    ));
    runner.drain();
    assert_eq!(runner.output, b"EOF");
    assert_eq!(runner.process.observe().unwrap(), None);
    runner.process.close().unwrap();
    assert!(matches!(
        runner.process.poll(Interest::Read, Duration::ZERO),
        Err(Failure::Closed)
    ));
}

#[test]
fn force() {
    let mut runner = Runner::new(support::shell(
        "trap '' HUP; printf READY; exec /bin/sleep 60",
    ));
    runner.until(b"READY");
    let before = Instant::now();
    let exit = runner.process.close().unwrap();
    assert_eq!(exit.signal, Some(9));
    assert!(before.elapsed() < Duration::from_secs(5));
    assert!(runner.process.status().closed());
    assert_eq!(runner.process.close().unwrap(), exit);
}
