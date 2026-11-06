#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

use bagua::ipc::{Connection, Service};
use protocol::{Command, Failure, Request, wire::Message};
use std::{
    fs,
    io::{self, Read, Write},
    os::unix::{fs::PermissionsExt, net::UnixStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

fn directory() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
    directory
}

#[test]
fn connection() {
    let directory = directory();
    let path = directory.path().join("host.sock");
    let mut service = Service::bind(&path).unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    let task = std::thread::spawn(move || service.run(&flag).unwrap());
    let mut connection = Connection::connect(&path).unwrap();
    assert!(matches!(
        connection.receive().unwrap(),
        Message::Hello {
            version: protocol::VERSION,
            ..
        }
    ));
    connection
        .send(&Request {
            version: 0,
            id: 1,
            command: Command::Capabilities,
        })
        .unwrap();
    assert!(
        matches!(connection.receive().unwrap(), Message::Reply(reply) if reply.result == Err(Failure::Version))
    );
    connection
        .send(&Request::identified(2, Command::create(80, 24).unwrap()).unwrap())
        .unwrap();
    assert!(matches!(connection.receive().unwrap(), Message::Reply(reply) if reply.result.is_ok()));
    assert!(matches!(
        connection.receive().unwrap(),
        Message::Finished {
            request: 2,
            result: Ok(())
        }
    ));
    connection.finish().unwrap();
    assert!(
        matches!(connection.receive(), Err(error) if error.kind() == io::ErrorKind::UnexpectedEof)
    );
    let mut next = Connection::connect(&path).unwrap();
    next.receive().unwrap();
    next.send(&Request::new(Command::Workspace(
        workspace::Operation::Read,
    )))
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
    assert_eq!(fs::read(&path).unwrap(), b"peer data");
}

#[test]
fn namespace() {
    let directory = directory();
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(Service::bind(&directory.path().join("other")).is_err());
}

#[test]
fn malformed() {
    let directory = directory();
    let path = directory.path().join("host.sock");
    let mut service = Service::bind(&path).unwrap();
    let stop = AtomicBool::new(false);
    let mut peer = UnixStream::connect(&path).unwrap();
    peer.write_all(&u32::MAX.to_be_bytes()).unwrap();
    for _ in 0..4 {
        service.step(&stop).unwrap();
    }
    let mut bytes = Vec::new();
    peer.read_to_end(&mut bytes).unwrap();
    assert!(bytes.len() < 256);
    let mut good = UnixStream::connect(&path).unwrap();
    good.write_all(
        &protocol::wire::encode(
            &Request::new(Command::Capabilities),
            protocol::wire::REQUEST,
        )
        .unwrap(),
    )
    .unwrap();
    for _ in 0..4 {
        service.step(&stop).unwrap();
    }
    let mut header = [0; 4];
    good.read_exact(&mut header).unwrap();
    assert!(u32::from_be_bytes(header) > 0);
}
