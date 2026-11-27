#![cfg(all(target_os = "windows", target_arch = "x86_64"))]

mod discovery;
mod inspect;
mod peer;

use bagua::ipc::{Connection, Service};
use protocol::{
    Command, Request,
    wire::{Message, Program},
};
use pty::Size;
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    process::{Command as Exec, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use workspace::{Geometry, Id, Operation};

struct Stop(Arc<AtomicBool>);

impl Drop for Stop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

#[test]
fn control() {
    let dir = peer::directory();
    let path = dir.path().join("host.sock");
    let mut service = Service::bind(&path).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let session = Id::new(1).unwrap();
    let commands = [
        Command::create(80, 24).unwrap(),
        Command::Workspace(Operation::Join {
            session,
            viewport: Geometry::new(80, 24).unwrap(),
        }),
        Command::Workspace(Operation::Acquire {
            session,
            fixed: None,
        }),
        Command::Start {
            pane: Id::new(3).unwrap(),
            program: Program {
                executable: peer::comspec(),
                args: vec!["/C".into(), "echo".into(), "ok".into()],
                cwd: dir.path().to_string_lossy().into_owned(),
                environment: BTreeMap::from([(
                    "SYSTEMROOT".into(),
                    peer::systemroot().to_string_lossy().into_owned(),
                )]),
            },
        },
    ];
    std::thread::scope(|scope| {
        let task = scope.spawn(|| service.run(&stop).unwrap());
        let guard = Stop(Arc::clone(&stop));
        let mut child = Exec::new(env!("CARGO_BIN_EXE_bagua"))
            .arg("--socket")
            .arg(&path)
            .args(["control", "--json"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        for (index, command) in commands.into_iter().enumerate() {
            let request = Request::identified(index as u64 + 1, command).unwrap();
            writeln!(input, "{}", serde_json::to_string(&request).unwrap()).unwrap();
        }
        drop(input);
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let messages: Vec<serde_json::Value> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert!(
            messages
                .iter()
                .any(|m| m["Finished"]["request"] == 4
                    && m["Finished"]["result"].get("Ok").is_some())
        );
        drop(guard);
        task.join().unwrap();
    });
    drop(service);
    assert!(!path.exists());
}

#[test]
fn serve() {
    let dir = peer::directory();
    let path = dir.path().join("host.sock");
    let child = Exec::new(env!("CARGO_BIN_EXE_bagua"))
        .args(["--no-config", "--socket"])
        .arg(&path)
        .arg("serve")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut connection = loop {
        match Connection::connect(&path) {
            Ok(connection) => break connection,
            Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            Err(error) => panic!("serve did not bind: {error}"),
        }
    };
    let hello = connection.receive().unwrap();
    assert!(matches!(
        hello,
        Message::Hello {
            identity: Some(_),
            ..
        }
    ));
    connection.send(&Request::new(Command::Shutdown)).unwrap();
    let _ = connection.receive();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!path.exists());
}

#[test]
fn unknown() {
    let dir = peer::directory();
    let path = dir.path().join("host.sock");
    fs::write(&path, "keep").unwrap();
    let output = Exec::new(env!("CARGO_BIN_EXE_bagua"))
        .args(["--no-config", "--socket"])
        .arg(&path)
        .args(["catalog", "--json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(fs::read_to_string(&path).unwrap(), "keep");
}

#[test]
fn interactive() {
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join("bagua");
    fs::create_dir_all(&config).unwrap();
    fs::write(
        config.join("bagua.toml"),
        "[header]\nenabled=false\ncomponents=['pane','command','cwd']\n[status]\ncomponents=['session','windows','prefix','notice','clock']\n",
    )
    .unwrap();
    let socket = home.path().join("bagua").join("runtime").join("host.sock");
    let mut first = peer::Peer::spawn(home.path(), &["enter", "work"], Size::new(100, 30).unwrap());
    first.until("control");
    assert!(bagua::ipc::private(socket.parent().unwrap()).unwrap());
    first.send(b"Write-Output 'BAGUA_WIN_OK'\r");
    first.until("BAGUA_WIN_OK");
    first.send(b"\x07v");
    let deadline = Instant::now() + Duration::from_secs(15);
    while peer::catalog(&socket)
        .sessions()
        .flat_map(|session| session.windows())
        .map(|window| window.panes().count())
        .sum::<usize>()
        != 2
    {
        assert!(Instant::now() < deadline, "split did not create two panes");
        first.pump();
        std::thread::sleep(Duration::from_millis(10));
    }
    first.send(b"\x07d");
    assert_eq!(first.finish(), 0);
    let snapshot = peer::catalog(&socket);
    let session = snapshot.sessions().next().unwrap();
    assert_eq!(session.controller(), None);
    let mut second = peer::Peer::spawn(
        home.path(),
        &["enter", "work", "--existing"],
        Size::new(100, 30).unwrap(),
    );
    second.until("control");
    second.send(b"\x07!");
    assert_eq!(second.finish(), 0);
    assert_eq!(peer::catalog(&socket).sessions().count(), 0);
    peer::shutdown(&socket);
}
