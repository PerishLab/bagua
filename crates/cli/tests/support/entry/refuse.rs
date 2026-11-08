use super::{Fixture, Peer, Size};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, MetadataExt},
};

#[test]
fn unknown() {
    let mut fixture = Fixture::cold();
    fixture.foreign();
    let path = fixture.path();
    let parent = path.parent().unwrap();
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(parent)
        .unwrap();
    fs::write(&path, "sentinel").unwrap();
    let inode = fs::metadata(&path).unwrap().ino();
    let mut peer = Peer::ordinary(&fixture, &[], Size::new(80, 24).unwrap());
    peer.until("RESTORED:2");
    assert_eq!(fs::metadata(&path).unwrap().ino(), inode);
    assert_eq!(fs::read_to_string(&path).unwrap(), "sentinel");
}

#[test]
fn image() {
    use std::io::{Read, Write};
    let mut fixture = Fixture::cold();
    fixture.foreign();
    let path = fixture.path();
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path.parent().unwrap())
        .unwrap();
    let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
    let inode = fs::metadata(&path).unwrap().ino();
    let metadata = fs::metadata(env!("CARGO_BIN_EXE_bagua")).unwrap();
    let message = protocol::wire::Message::Hello {
        version: protocol::VERSION,
        client: workspace::Id::new(1).unwrap(),
        canvas: workspace::Geometry::new(80, 64).unwrap(),
        identity: Some(protocol::wire::Image {
            device: metadata.dev(),
            inode: metadata.ino() + 1,
        }),
    };
    let task = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .unwrap();
        stream
            .write_all(&protocol::wire::encode(&message, protocol::wire::FRAME).unwrap())
            .unwrap();
        let mut buffer = [0; 16];
        assert_eq!(stream.read(&mut buffer).unwrap(), 0);
    });
    let mut peer = Peer::ordinary(&fixture, &[], Size::new(80, 24).unwrap());
    peer.until("RESTORED:2");
    assert!(peer.text().contains("different executable file"));
    assert_eq!(fs::metadata(&path).unwrap().ino(), inode);
    task.join().unwrap();
}

#[test]
fn absent() {
    let mut fixture = Fixture::cold();
    fixture.foreign();
    let mut peer = Peer::ordinary(
        &fixture,
        &["enter", "--observe"],
        Size::new(80, 24).unwrap(),
    );
    peer.until("RESTORED:2");
    assert!(!fixture.path().parent().unwrap().exists());
}

#[test]
fn invalid() {
    let mut fixture = Fixture::cold();
    fixture.foreign();
    let mut peer = Peer::ordinary(&fixture, &["enter", " bad "], Size::new(80, 24).unwrap());
    peer.until("RESTORED:2");
    assert!(peer.text().contains("name"));
    assert!(!fixture.path().parent().unwrap().exists());
}
