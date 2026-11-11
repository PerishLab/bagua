use super::super::interactive::{Fixture, Peer, SHELL};
use pty::Size;
use std::fs;
use terminal::Point;

#[test]
fn defaults() {
    let fixture = Fixture::new();
    let mut args = vec!["--no-config"];
    args.extend_from_slice(SHELL);
    let mut peer = Peer::new(&fixture, &args, Size::new(100, 24).unwrap());
    peer.until("W1:sh");
    peer.send(b"printf 'DEFAULT-'; stty size\r");
    peer.until("DEFAULT-21 100");
    let catalog = fixture.catalog();
    let session = catalog.sessions().next().unwrap();
    assert!(session.header());
    assert_eq!(session.geometry().rows(), 22);
    assert!(!peer.text().contains("bagua w"));
    peer.detach();
}

fn config(position: &str) -> String {
    format!(
        "[header]\nenabled=true\nposition='bottom'\nformat=' P{{{{pane.index}}}} {{{{pane.command}}}} '\n[status]\nposition='{position}'\n[[status.lines]]\nleft='ROW {{{{session.name}}}}'\nright='{{{{clock.time}}}}'\n"
    )
}

#[test]
fn reload() {
    let fixture = Fixture::new();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("bars.toml");
    fs::write(&path, config("top")).unwrap();
    let mut args = vec!["--config", path.to_str().unwrap()];
    args.extend_from_slice(SHELL);
    let mut peer = Peer::new(&fixture, &args, Size::new(100, 24).unwrap());
    peer.until("P1 sh");
    peer.until("ROW");
    assert_eq!(
        peer.engine
            .snapshot()
            .cell(Point { x: 1, y: 23 })
            .unwrap()
            .text,
        "P"
    );
    peer.send(b"printf 'TOP-'; stty size\r");
    peer.until("TOP-22 100");
    let pane = workspace::Id::new(3).unwrap();
    let pid = fixture.context(pane).pid;
    fs::write(
        &path,
        config("top").replace("{{pane.command}}", "{{unknown}}"),
    )
    .unwrap();
    peer.send(b"\x07r");
    peer.until("Configuration unchanged");
    assert_eq!(fixture.context(pane).pid, pid);
    fs::write(&path, config("bottom")).unwrap();
    peer.send(b"\x07r");
    peer.until("Configuration reloaded");
    peer.until("ROW");
    assert_eq!(
        peer.engine
            .snapshot()
            .cell(Point { x: 1, y: 22 })
            .unwrap()
            .text,
        "P"
    );
    assert_eq!(fixture.context(pane).pid, pid);
    peer.send(b"printf 'BOTTOM-'; stty size\r");
    peer.until("BOTTOM-22 100");
    peer.detach();
}
