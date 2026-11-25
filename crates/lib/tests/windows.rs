#![cfg(all(target_os = "windows", target_arch = "x86_64"))]

use bagua::ipc::{Connection, Service, private, secure};
use protocol::{Command, Failure, Input, Request, wire::Message};
use std::{
    fs, io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use workspace::{Id, Operation};

fn id(value: u64) -> Id {
    Id::new(value).unwrap()
}

fn directory() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    secure(directory.path()).unwrap();
    directory
}

#[test]
fn defaults() {
    let config = bagua::config::Config::default();
    assert_eq!(config.workspace.cwd, "launch");
}

#[test]
fn connection() {
    let directory = directory();
    let path = directory.path().join("host.sock");
    let mut service = Service::bind(&path).unwrap();
    assert!(path.exists());
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    let task = std::thread::spawn(move || service.run(&flag).unwrap());
    let mut peer = Connection::connect(&path).unwrap();
    assert!(matches!(
        peer.receive().unwrap(),
        Message::Hello {
            version: protocol::VERSION,
            ..
        }
    ));
    peer.send(&Request {
        version: 0,
        id: 1,
        command: Command::Capabilities,
    })
    .unwrap();
    assert!(
        matches!(peer.receive().unwrap(), Message::Reply(reply) if reply.result == Err(Failure::Version))
    );
    peer.send(&Request::identified(2, Command::create(80, 24).unwrap()).unwrap())
        .unwrap();
    assert!(matches!(peer.receive().unwrap(), Message::Reply(reply) if reply.result.is_ok()));
    assert!(matches!(
        peer.receive().unwrap(),
        Message::Finished {
            request: 2,
            result: Ok(())
        }
    ));
    peer.finish().unwrap();
    assert!(matches!(peer.receive(), Err(error) if error.kind() == io::ErrorKind::UnexpectedEof));
    let mut next = Connection::connect(&path).unwrap();
    next.receive().unwrap();
    next.send(&Request::new(Command::Workspace(Operation::Read)))
        .unwrap();
    assert!(matches!(next.receive().unwrap(), Message::Reply(reply) if reply.result.is_ok()));
    stop.store(true, Ordering::Relaxed);
    task.join().unwrap();
    assert!(!path.exists());
}

#[test]
fn occupied() {
    let directory = directory();
    let path = directory.path().join("occupied");
    fs::write(&path, b"peer data").unwrap();
    assert!(Service::bind(&path).is_err());
    let error = Connection::connect(&path).err().unwrap();
    assert_eq!(error.kind(), io::ErrorKind::ConnectionRefused);
    assert_eq!(fs::read(&path).unwrap(), b"peer data");
}

#[test]
fn namespace() {
    let open = tempfile::tempdir().unwrap();
    assert!(!private(open.path()).unwrap());
    assert!(Service::bind(&open.path().join("host.sock")).is_err());
    secure(open.path()).unwrap();
    assert!(private(open.path()).unwrap());
    let service = Service::bind(&open.path().join("host.sock")).unwrap();
    drop(service);
    assert!(!open.path().join("host.sock").exists());
}

#[test]
fn channel() {
    let directory = directory();
    let path = directory.path().join("channel.sock");
    let mut service = Service::bind(&path).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    let task = std::thread::spawn(move || service.run(&flag).unwrap());
    let mut client = Connection::connect(&path).unwrap();
    client.receive().unwrap();
    let mut channel = client.channel().unwrap();
    channel
        .send(&Request::identified(1, Command::Capabilities).unwrap())
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut replied = false;
    while !replied {
        assert!(Instant::now() < deadline);
        channel.flush().unwrap();
        if let Some(Message::Reply(reply)) = channel.receive().unwrap() {
            assert_eq!(reply.request, 1);
            assert!(reply.result.is_ok());
            replied = true;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    let big = Request::new(Command::Send {
        pane: id(1),
        input: Input::Text("x".repeat(100_000)),
    });
    channel.send(&big).unwrap();
    channel.send(&big).unwrap();
    assert_eq!(
        channel.send(&big).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    stop.store(true, Ordering::Relaxed);
    task.join().unwrap();
}
