use super::interactive::{Fixture, Peer, SHELL};
use pty::Size;
use std::{
    fs,
    time::{Duration, Instant},
};

#[test]
fn clipboard() {
    copying("local");
    copying("osc52");
}

fn copying(target: &str) {
    let fixture = Fixture::new();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("bagua.toml");
    let script =
        r"stty raw -echo; printf '\033]52;c;dW50cnVzdGVk\033\134hello'; exec /bin/sleep 60";
    fs::write(&path, format!("[clipboard]\ntarget='{target}'\n[program]\nexecutable='/bin/sh'\nargs=['-c', '''{script}''']\ncwd='/'\nenvironment={{PATH='/usr/bin:/bin',TERM='xterm-256color'}}\n")).unwrap();
    let mut peer = Peer::new(
        &fixture,
        &["--config", path.to_str().unwrap(), "open"],
        Size::new(80, 24).unwrap(),
    );
    peer.until("hello");
    assert!(!peer.output.windows(5).any(|bytes| bytes == b"\x1b]52;"));
    peer.send(b"\x1b[<0;1;1M\x1b[<32;5;1M\x1b[<0;5;1m");
    peer.until("Copied 5 bytes locally");
    peer.detach();
    let copies: Vec<_> = peer
        .output
        .windows(5)
        .enumerate()
        .filter_map(|(index, bytes)| (bytes == b"\x1b]52;").then_some(index))
        .collect();
    if target == "osc52" {
        assert_eq!(copies.len(), 1);
        let start = copies[0];
        assert_eq!(&peer.output[start..start + 17], b"\x1b]52;c;aGVsbG8=\x1b\\");
    } else {
        assert!(copies.is_empty());
    }
    assert_eq!(peer.engine.modes().keyboard, 0);
}

#[test]
fn divider() {
    let fixture = Fixture::new();
    let mut peer = Peer::new(&fixture, SHELL, Size::new(80, 24).unwrap());
    peer.until("control");
    peer.send(b"\x07vprintf '\\123\\120\\114\\111\\124\\122\\105\\101\\104\\131\\n'\r");
    peer.until("SPLITREADY");
    peer.send(b"\x1b[<0;40;2M\x1b[<32;51;2M\x1b[<0;51;2m");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let catalog = fixture.catalog();
        let session = catalog.sessions().next().unwrap();
        let places = session
            .windows()
            .next()
            .unwrap()
            .placements(session.geometry())
            .unwrap();
        if places[0].rect.geometry.columns() == 50 {
            break;
        }
        assert!(Instant::now() < deadline, "divider was not applied");
        peer.pump();
    }
    peer.send(b"printf 'RESIZED-'; stty size\r");
    peer.until("RESIZED-23 29");
    assert_eq!(
        fixture.view(workspace::Id::new(3).unwrap()).extent.width(),
        50
    );
    peer.detach();
}
