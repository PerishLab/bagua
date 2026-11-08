#[path = "entry/race.rs"]
mod race;
#[path = "entry/refuse.rs"]
mod refuse;
use crate::interactive::{Fixture, Peer};
use pty::Size;
use std::{
    fs,
    os::unix::fs::MetadataExt,
    time::{Duration, Instant},
};

fn panes(fixture: &Fixture) -> usize {
    fixture
        .catalog()
        .sessions()
        .flat_map(|session| session.windows())
        .map(|window| window.panes().count())
        .sum()
}

fn running(fixture: &Fixture) -> bool {
    fixture
        .catalog()
        .sessions()
        .flat_map(|session| session.windows())
        .flat_map(|window| window.panes())
        .all(|pane| pane.state() == workspace::State::Running)
}

fn wait(peer: &mut Peer, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done() {
        assert!(
            Instant::now() < deadline,
            "workspace did not reach expected state"
        );
        peer.pump();
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn quoted(path: &std::path::Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

#[test]
fn ordinary() {
    let mut fixture = Fixture::cold();
    let mut peer = Peer::ordinary(&fixture, &[], Size::new(80, 24).unwrap());
    peer.until("bagua w1 control");
    let snapshot = fixture.catalog();
    let session = snapshot.sessions().next().unwrap();
    assert_eq!(session.name(), "workspace");
    assert_eq!(session.windows().count(), 1);
    assert!(session.controller().is_some());
    assert_eq!(
        fs::metadata(fixture.path().parent().unwrap())
            .unwrap()
            .mode()
            & 0o777,
        0o700
    );
    assert!(fixture.shutdown());
    peer.until("RESTORED:2");
    assert!(!peer.text().contains("BROKEN-MODE"));
}

#[test]
fn cwd() {
    let mut fixture = Fixture::cold();
    let a = fixture.path().parent().unwrap().join("a");
    let b = fixture.path().parent().unwrap().join("b");
    let mut peer = Peer::ordinary(&fixture, &[], Size::new(80, 24).unwrap());
    peer.until("bagua w1 control");
    fs::create_dir(&a).unwrap();
    fs::create_dir(&b).unwrap();
    peer.send(format!("cd {}; printf '\\103\\127\\104_READY\\n'\n", quoted(&a)).as_bytes());
    peer.until("CWD_READY");
    peer.send(b"\x07\\");
    wait(&mut peer, || panes(&fixture) == 2 && running(&fixture));
    let snapshot = fixture.catalog();
    let session = snapshot.sessions().next().unwrap();
    let pane = session.windows().next().unwrap().focus();
    assert_eq!(fixture.context(pane).cwd, a.to_str().unwrap());
    let deadline = Instant::now() + Duration::from_secs(5);
    while peer.engine.snapshot().cursor().position.x < 40 {
        assert!(Instant::now() < deadline);
        peer.pump();
        std::thread::sleep(Duration::from_millis(2));
    }
    peer.send(
        format!(
            "cd {}; printf '\\116\\105\\130\\124_READY\\n'\n",
            quoted(&b)
        )
        .as_bytes(),
    );
    peer.until("NEXT_READY");
    peer.send(b"\x07=");
    wait(&mut peer, || {
        fixture
            .catalog()
            .sessions()
            .next()
            .unwrap()
            .windows()
            .count()
            == 2
            && running(&fixture)
    });
    let snapshot = fixture.catalog();
    let session = snapshot.sessions().next().unwrap();
    let window = session
        .windows()
        .find(|window| Some(window.id()) == session.selected())
        .unwrap();
    assert_eq!(fixture.context(window.focus()).cwd, b.to_str().unwrap());
    peer.until("bagua w2 control");
    peer.send(b"\x1b1");
    wait(&mut peer, || {
        fixture.catalog().sessions().next().unwrap().selected()
            == Some(workspace::Id::new(2).unwrap())
    });
    peer.until("bagua w1 control");
    peer.send(format!("rmdir {}; printf '\\104\\105\\114_READY\\n'\n", quoted(&b)).as_bytes());
    peer.until("DEL_READY");
    let before = panes(&fixture);
    peer.send(b"\x07-");
    peer.until("Inspect");
    assert_eq!(panes(&fixture), before);
    assert!(fixture.shutdown());
    peer.until("RESTORED:2");
}
