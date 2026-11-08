use crate::interactive::{Fixture, Peer};
use pty::Size;

#[test]
fn browsing() {
    let fixture = Fixture::new();
    let script = "i=0; while [ $i -lt 120 ]; do printf 'row%03d\\r\\n' $i; i=$((i+1)); done; printf TAIL; exec /bin/sleep 60";
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
            script,
            "--env",
            "PATH=/usr/bin:/bin",
            "--env",
            "TERM=xterm-256color",
        ],
        Size::new(80, 24).unwrap(),
    );
    peer.until("TAIL");
    peer.send(b"\x07\x1b[5~");
    peer.until("History");
    assert!(!peer.text().contains("TAIL"));
    peer.send(b"\x1b[F");
    peer.until("TAIL");
    peer.send(b"\x1b[<64;2;2M");
    peer.until("History");
    assert!(!peer.text().contains("TAIL"));
    peer.send(b"\x1b[<65;2;2M");
    peer.until("TAIL");
    peer.detach();
    assert_eq!(fixture.catalog().sessions().count(), 1);
}
