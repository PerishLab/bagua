use super::interactive::{Fixture, Peer, legacy};
use pty::Size;
use std::fs;

fn contents(prefix: &str) -> String {
    legacy(&format!(
        "version=1\n[client]\nprefix='{prefix}'\nescape_ms=20\n[program]\nexecutable='/bin/sh'\nargs=['-i']\ncwd='/'\n[program.environment]\nPATH='/usr/bin:/bin'\nTERM='xterm-256color'\n"
    ))
}

#[test]
fn live() {
    let fixture = Fixture::new();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("bagua.toml");
    fs::write(&path, contents("C-q")).unwrap();
    let mut peer = Peer::new(
        &fixture,
        &["--config", path.to_str().unwrap(), "open"],
        Size::new(120, 28).unwrap(),
    );
    peer.until("control");
    peer.send(b"printf 'CONFIG_%s\\n' LIVE\r");
    peer.until("CONFIG_LIVE");
    let before = fixture.catalog();
    fs::write(&path, contents("C-a").replace("version=1", "version=2")).unwrap();
    peer.send(b"\x11r");
    peer.until("Configuration unchanged");
    peer.send(b"\x11?");
    peer.until("Prefix:");
    fs::write(&path, contents("C-a")).unwrap();
    peer.send(b"\x11r");
    peer.until("Configuration reloaded");
    let after = fixture.catalog();
    let old = before.sessions().next().unwrap();
    let new = after.sessions().next().unwrap();
    assert_eq!(old.id(), new.id());
    assert_eq!(old.controller(), new.controller());
    assert_eq!(
        old.windows().next().unwrap().id(),
        new.windows().next().unwrap().id()
    );
    assert!(
        new.windows()
            .flat_map(|window| window.panes())
            .all(|pane| pane.state() == workspace::State::Running)
    );
    peer.send(b"\x01d");
    peer.until("RESTORED:0");
    assert!(
        fixture
            .catalog()
            .sessions()
            .next()
            .unwrap()
            .controller()
            .is_none()
    );
}

#[test]
fn precedence() {
    let fixture = Fixture::new();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("bagua.toml");
    fs::write(&path, contents("C-q").replace("/bin/sh", "/does/not/exist")).unwrap();
    let mut peer = Peer::new(
        &fixture,
        &[
            "--config",
            path.to_str().unwrap(),
            "open",
            "--executable",
            "/bin/sh",
            "--cwd",
            "/",
            "--arg=-i",
            "--env",
            "PATH=/usr/bin:/bin",
        ],
        Size::new(80, 24).unwrap(),
    );
    peer.until("control");
    peer.send(b"printf 'OVERRIDE_%s\\n' LIVE\r");
    peer.until("OVERRIDE_LIVE");
    peer.send(b"\x11d");
    peer.until("RESTORED:0");
}
