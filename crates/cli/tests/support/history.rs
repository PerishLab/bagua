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

#[test]
fn copying() {
    let fixture = Fixture::new();
    let mut peer = Peer::new(
        &fixture,
        super::interactive::SHELL,
        Size::new(100, 24).unwrap(),
    );
    peer.until("control");
    peer.send(b"i=0; while [ $i -lt 120 ]; do printf 'row%03d\\r\\n' $i; i=$((i+1)); done; printf '\\103\\117\\120\\131_READY\\n'\r");
    peer.until("COPY_READY");
    peer.send(b"\x07\x1b[5~");
    peer.until("History");
    peer.send(
        b"\x1b[<0;1;1M\x1b[<32;5;1M\x1b[<0;5;1mprintf '\\101\\106\\124\\105\\122_COPY_OK\\n'\r",
    );
    peer.until("AFTER_COPY_OK");
    peer.send(b"\x07\x1b[5~");
    peer.until("History");
    peer.send(b"\x07[\x07cprintf '\\113\\105\\131_COPY_OK\\n'\r");
    peer.until("KEY_COPY_OK");
    peer.send(b"\x07\x1b[5~");
    peer.until("History");
    peer.send(b"\x07[\x1b[27uprintf '\\105\\123\\103_LIVE_OK\\n'\r");
    peer.until("ESC_LIVE_OK");
    let screen = fixture.view(workspace::Id::new(3).unwrap());
    let text: String = screen.cells.iter().map(|cell| cell.text.as_str()).collect();
    for marker in ["AFTER_COPY_OK", "KEY_COPY_OK", "ESC_LIVE_OK"] {
        assert!(text.contains(marker));
    }
    peer.detach();
}
