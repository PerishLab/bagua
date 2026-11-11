use super::interactive::{Fixture, Peer, legacy};
use pty::Size;
use std::fs;

#[test]
fn negotiated() {
    negotiate(1);
    negotiate(5);
    negotiate(7);
}

fn negotiate(flags: u16) {
    let fixture = Fixture::new();
    let script = r"stty raw -echo; printf '\033[>1u\033[?u\033[c'; od -An -tx1 -N12; printf '\r\nNEGOTIATED\r\n'; od -An -tx1 -N8; printf '\r\nDISTINCT\r\n'; printf '\033[<u'; od -An -tx1 -N1; printf '\r\nLEGACY\r\n'; exec /bin/sleep 60";
    let script = script.replace(">1u", &format!(">{flags}u"));
    let mut peer = Peer::new(
        &fixture,
        &[
            "open",
            "--executable",
            "/bin/sh",
            "--cwd",
            "/",
            "--arg=-c",
            "--arg",
            &script,
            "--env",
            "PATH=/usr/bin:/bin",
            "--env",
            "TERM=xterm-256color",
        ],
        Size::new(120, 28).unwrap(),
    );
    peer.until("NEGOTIATED");
    assert!(peer.text().contains("1b 5b 3f 31 75 1b 5b 3f 31 3b 32 63"));
    peer.send(b"\r\x1b[13;2u");
    peer.until("DISTINCT");
    assert!(peer.text().contains("0d 1b 5b 31 33 3b 32 75"));
    peer.send(b"\x07\x07");
    peer.until("LEGACY");
    assert!(peer.text().contains("07"));
    peer.detach();
    assert_eq!(peer.engine.modes().keyboard, 0);
}

#[test]
fn bindings() {
    let fixture = Fixture::new();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("bagua.toml");
    fs::write(&path, legacy("[client]\nprefix='C-q'\n[bindings.prefix]\n's'='split-rows'\n[bindings.direct]\n'Alt-q'='detach'\n[program]\nexecutable='/bin/sh'\nargs=['-i']\ncwd='/'\nenvironment={PATH='/usr/bin:/bin',TERM='xterm-256color'}\n")).unwrap();
    let mut peer = Peer::new(
        &fixture,
        &["--config", path.to_str().unwrap(), "open"],
        Size::new(80, 24).unwrap(),
    );
    peer.until("control");
    peer.send(b"\x11");
    peer.until("prefix pending");
    peer.send(b"s");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while fixture
        .catalog()
        .sessions()
        .next()
        .unwrap()
        .windows()
        .next()
        .unwrap()
        .panes()
        .count()
        != 2
    {
        assert!(std::time::Instant::now() < deadline);
        peer.pump();
    }
    let text = fs::read_to_string(&path).unwrap();
    fs::write(&path, text.replace("split-rows", "unknown")).unwrap();
    peer.send(b"\x11r");
    peer.until("Configuration unchanged");
    fs::write(&path, text.replace("split-rows", "window")).unwrap();
    peer.send(b"\x11r");
    peer.until("Configuration reloaded");
    peer.send(b"\x11s");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while fixture
        .catalog()
        .sessions()
        .next()
        .unwrap()
        .windows()
        .count()
        != 2
    {
        assert!(std::time::Instant::now() < deadline);
        peer.pump();
    }
    peer.send(b"\x1bq");
    peer.until("RESTORED:0");
    assert_eq!(peer.engine.modes().keyboard, 0);
}
