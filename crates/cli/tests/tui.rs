#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "support/configure.rs"]
mod configure;
#[path = "support/entry.rs"]
mod entry;
#[path = "support/history.rs"]
mod history;
#[path = "support/interactive.rs"]
mod interactive;
#[path = "support/keyboard.rs"]
mod keyboard;
use interactive::{Fixture, Peer, SHELL};
use pty::Size;
use std::time::{Duration, Instant};

#[test]
fn recovery() {
    let fixture = Fixture::new();
    let mut owner = Peer::new(&fixture, SHELL, Size::new(80, 24).unwrap());
    owner.until("control");
    owner.send(b"\x07v");
    let deadline = Instant::now() + Duration::from_secs(5);
    while fixture
        .catalog()
        .sessions()
        .flat_map(|s| s.windows())
        .map(|w| w.panes().count())
        .sum::<usize>()
        != 2
    {
        assert!(Instant::now() < deadline);
        owner.pump();
    }
    owner.detach();
    let mut failed = Peer::new(
        &fixture,
        &["connect", "1", "--take"],
        Size::new(4, 10).unwrap(),
    );
    failed.until("RESTORED:2");
    let mut fixed = Peer::new(
        &fixture,
        &["connect", "1", "--take", "--fixed"],
        Size::new(4, 10).unwrap(),
    );
    fixed.until("bagu");
    assert!(
        matches!(fixture.catalog().sessions().next().unwrap().sizing(),workspace::Sizing::Fixed(size) if size.columns()==80 && size.rows()==23)
    );
    fixed.detach();
}

#[test]
fn paste() {
    let fixture = Fixture::new();
    let script = r"stty raw -echo; printf '\033[?2004h\033[?1000h\033[?1006hREADY'; od -An -tx1 -N17; printf '\r\nPASTED\r\n'; od -An -tx1 -N9; printf '\r\nMOUSED\r\n'; exec /bin/sleep 60";
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
    peer.until("READY");
    peer.send(b"\x1b[<4;1;1M\x1b[<36;5;1M\x1b[<4;5;1m\x07c\x07p");
    peer.until("PASTED");
    assert!(peer.text().contains("52 45 41 44 59"));
    peer.send(b"\x1b[<0;2;2M");
    peer.until("MOUSED");
    assert!(peer.text().contains("3c 30 3b 32 3b 32 4d"));
    peer.detach();
}

#[test]
fn failure() {
    let fixture = Fixture::new();
    let mut peer = Peer::new(&fixture, &["connect", "999"], Size::new(80, 24).unwrap());
    peer.until("RESTORED:2");
    assert!(!peer.text().contains("BROKEN-MODE"));
}

#[test]
fn crash() {
    let fixture = Fixture::new();
    let mut peer = Peer::new(&fixture, SHELL, Size::new(80, 24).unwrap());
    peer.until("control");
    peer.process.close().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let snapshot = fixture.catalog();
        let session = snapshot.sessions().next().unwrap();
        if session.controller().is_none() {
            assert!(
                session
                    .windows()
                    .flat_map(|w| w.panes())
                    .all(|p| p.state() == workspace::State::Running)
            );
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn workflow() {
    let fixture = Fixture::new();
    let mut peer = Peer::new(&fixture, SHELL, Size::new(80, 24).unwrap());
    peer.until("control");
    peer.send(b"printf 'BAGUA_%s\\n' ONE\r");
    peer.until("BAGUA_ONE");
    peer.send(b"\x07v");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        peer.pump();
        let count: usize = fixture
            .catalog()
            .sessions()
            .flat_map(|s| s.windows())
            .map(|w| w.panes().count())
            .sum();
        if count == 2 {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    peer.send(b"\x07n");
    let deadline = Instant::now() + Duration::from_secs(10);
    while fixture
        .catalog()
        .sessions()
        .map(|s| s.windows().count())
        .sum::<usize>()
        != 2
    {
        assert!(Instant::now() < deadline);
        peer.pump();
        std::thread::sleep(Duration::from_millis(5));
    }
    peer.detach();
    let snapshot = fixture.catalog();
    let session = snapshot.sessions().next().unwrap();
    assert_eq!(session.controller(), None);
    assert!(
        session
            .windows()
            .flat_map(|w| w.panes())
            .all(|p| p.state() == workspace::State::Running)
    );
    let mut next = Peer::new(
        &fixture,
        &["connect", "1", "--take"],
        Size::new(80, 24).unwrap(),
    );
    next.until("control");
    next.send(b"\x07x");
    let deadline = Instant::now() + Duration::from_secs(5);
    while fixture
        .catalog()
        .sessions()
        .map(|s| s.windows().count())
        .sum::<usize>()
        != 1
    {
        assert!(Instant::now() < deadline);
        next.pump();
        std::thread::sleep(Duration::from_millis(2));
    }
    next.send(b"\x07!");
    next.until("RESTORED:0");
    assert_eq!(fixture.catalog().sessions().count(), 0);
}

#[test]
fn observers() {
    let fixture = Fixture::new();
    let mut owner = Peer::new(&fixture, SHELL, Size::new(80, 24).unwrap());
    owner.until("control");
    let mut observer = Peer::new(&fixture, &["connect", "1"], Size::new(40, 12).unwrap());
    observer.until("observe");
    let size = fixture.catalog().sessions().next().unwrap().geometry();
    assert_eq!((size.columns(), size.rows()), (80, 23));
    observer.send(b"X");
    observer.until("requires");
    owner.send(b"\x07t");
    observer.until("control");
    owner.until("observe");
    let size = fixture.catalog().sessions().next().unwrap().geometry();
    assert_eq!((size.columns(), size.rows()), (40, 11));
    observer.send(b"\x07f");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !matches!(
        fixture.catalog().sessions().next().unwrap().sizing(),
        workspace::Sizing::Fixed(_)
    ) {
        assert!(Instant::now() < deadline);
        observer.pump();
        owner.pump();
    }
    observer.process.resize(Size::new(60, 16).unwrap()).unwrap();
    observer
        .engine
        .resize(terminal::Extent::new(60, 16).unwrap())
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        observer.pump();
        owner.pump();
        if matches!(
            fixture.catalog().sessions().next().unwrap().sizing(),
            workspace::Sizing::Fixed(_)
        ) {
            break;
        }
        assert!(Instant::now() < deadline);
    }
    let size = fixture.catalog().sessions().next().unwrap().geometry();
    assert_eq!((size.columns(), size.rows()), (40, 11));
    observer.detach();
    owner.detach();
}
