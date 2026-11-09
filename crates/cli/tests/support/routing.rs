use super::interactive::{Fixture, Peer, SHELL};
use pty::Size;
use workspace::Id;

fn text(fixture: &Fixture, pane: u64) -> String {
    fixture
        .view(Id::new(pane).unwrap())
        .cells
        .iter()
        .map(|cell| cell.text.as_str())
        .collect()
}

#[test]
fn split() {
    let fixture = Fixture::new();
    let mut peer = Peer::new(&fixture, SHELL, Size::new(80, 24).unwrap());
    peer.until("control");
    peer.send(b"\x07\\printf '\\106\\101\\123\\124_ROUTE_OK\\n'\r");
    peer.until("FAST_ROUTE_OK");
    assert!(!text(&fixture, 3).contains("FAST_ROUTE_OK"));
    assert!(text(&fixture, 4).contains("FAST_ROUTE_OK"));
    peer.detach();
}

#[test]
fn chain() {
    let fixture = Fixture::new();
    let mut peer = Peer::new(&fixture, SHELL, Size::new(80, 24).unwrap());
    peer.until("control");
    peer.send(b"\x07\\\x07=printf '\\127\\111\\116_ROUTE_OK\\n'\r\x1b1printf '\\102\\101\\103\\113_ROUTE_OK\\n'\r");
    peer.until("BACK_ROUTE_OK");
    assert!(!text(&fixture, 3).contains("ROUTE_OK"));
    assert!(text(&fixture, 4).contains("BACK_ROUTE_OK"));
    assert!(!text(&fixture, 4).contains("WIN_ROUTE_OK"));
    assert!(text(&fixture, 6).contains("WIN_ROUTE_OK"));
    assert!(!text(&fixture, 6).contains("BACK_ROUTE_OK"));
    peer.detach();
}

#[test]
fn click() {
    let fixture = Fixture::new();
    let mut peer = Peer::new(&fixture, SHELL, Size::new(80, 24).unwrap());
    peer.until("control");
    peer.send(b"\x07\\printf '\\116\\105\\127_READY\\n'\r");
    peer.until("NEW_READY");
    peer.send(b"\x1b[<0;1;1M\x1b[<0;1;1mprintf '\\103\\114\\111\\103\\113_ROUTE_OK\\n'\r");
    peer.until("CLICK_ROUTE_OK");
    assert!(text(&fixture, 3).contains("CLICK_ROUTE_OK"));
    assert!(!text(&fixture, 4).contains("CLICK_ROUTE_OK"));
    peer.detach();
}

#[test]
fn refusal() {
    let fixture = Fixture::new();
    let mut peer = Peer::new(&fixture, SHELL, Size::new(100, 3).unwrap());
    peer.until("control");
    peer.send(b"\x07-printf '\\102\\101\\104_ROUTE_OK\\n'\r");
    peer.until("Split top/bottom:");
    assert!(!text(&fixture, 3).contains("BAD_ROUTE_OK"));
    peer.send(b"printf '\\107\\117\\117\\104_ROUTE_OK'\r");
    peer.until("GOOD_ROUTE_OK");
    assert!(!text(&fixture, 3).contains("BAD_ROUTE_OK"));
    assert!(text(&fixture, 3).contains("GOOD_ROUTE_OK"));
    peer.detach();
}

#[test]
fn close() {
    let fixture = Fixture::new();
    let mut peer = Peer::new(&fixture, SHELL, Size::new(80, 24).unwrap());
    peer.until("control");
    peer.send(b"\x07=printf '\\127\\111\\116_READY\\n'\r");
    peer.until("WIN_READY");
    peer.send(b"\x07xprintf '\\106\\101\\114\\114\\102\\101\\103\\113_ROUTE_OK\\n'\r");
    peer.until("FALLBACK_ROUTE_OK");
    assert!(text(&fixture, 3).contains("FALLBACK_ROUTE_OK"));
    assert_eq!(
        fixture
            .catalog()
            .sessions()
            .next()
            .unwrap()
            .windows()
            .count(),
        1
    );
    peer.detach();
}

#[test]
fn navigation() {
    let fixture = Fixture::new();
    let mut peer = Peer::new(&fixture, SHELL, Size::new(100, 24).unwrap());
    peer.until("control");
    peer.send(b"\x1b9");
    peer.until("No window 9");
    peer.send(b"\x1b9\x1b1\x07\t\x07wprintf '\\116\\101\\126_OK\\n'\r");
    peer.until("NAV_OK");
    assert!(text(&fixture, 3).contains("NAV_OK"));
    assert_eq!(
        fixture
            .catalog()
            .sessions()
            .next()
            .unwrap()
            .windows()
            .count(),
        1
    );
    peer.detach();
}
