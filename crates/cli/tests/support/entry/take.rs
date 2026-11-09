use crate::interactive::{Fixture, Peer, SHELL};
use pty::Size;
use workspace::Id;

fn owner(fixture: &Fixture) -> Option<Id> {
    fixture.catalog().sessions().next().unwrap().controller()
}

#[test]
fn ordinary() {
    let fixture = Fixture::cold();
    let mut old = Peer::ordinary(&fixture, &[], Size::new(80, 24).unwrap());
    old.until("control");
    old.send(
        b"bagua_cookie=Kept; export bagua_cookie; printf '\\117\\127\\116\\105\\122_READY\\n'\r",
    );
    old.until("OWNER_READY");
    let pane = Id::new(3).unwrap();
    let pid = fixture.context(pane).pid;
    let previous = owner(&fixture);
    let mut next = Peer::ordinary(&fixture, &["--take"], Size::new(100, 26).unwrap());
    next.until("control");
    old.until("RESTORED:0");
    assert_ne!(owner(&fixture), previous);
    assert_eq!(fixture.context(pane).pid, pid);
    next.send(b"printf '\\123\\125\\122\\126\\111\\126\\105\\104_%s\\n' \"$bagua_cookie\"\r");
    next.until("SURVIVED_Kept");
    assert_eq!(
        fixture
            .catalog()
            .sessions()
            .next()
            .unwrap()
            .clients()
            .count(),
        1
    );
    next.detach();
    assert!(owner(&fixture).is_none());
}

#[test]
fn refusal() {
    let fixture = Fixture::cold();
    let mut old = Peer::ordinary(&fixture, &[], Size::new(80, 24).unwrap());
    old.until("control");
    old.send(b"\x07-printf '\\124\\127\\117_READY\\n'\r");
    old.until("TWO_READY");
    let previous = owner(&fixture);
    let pane = Id::new(4).unwrap();
    let pid = fixture.context(pane).pid;
    let mut next = Peer::ordinary(&fixture, &["enter", "--take"], Size::new(40, 3).unwrap());
    next.until("RESTORED:2");
    assert_eq!(owner(&fixture), previous);
    assert_eq!(fixture.context(pane).pid, pid);
    old.send(b"printf '\\117\\127\\116\\105\\122_STILL_OK\\n'\r");
    old.until("OWNER_STILL_OK");
    old.detach();
}

#[test]
fn choice() {
    let fixture = Fixture::cold();
    let mut old = Peer::ordinary(&fixture, &[], Size::new(80, 24).unwrap());
    old.until("control");
    let previous = owner(&fixture);
    let mut observer = Peer::ordinary(&fixture, &[], Size::new(100, 24).unwrap());
    observer.until("[t] Take");
    observer.send(b"o");
    observer.until("observe");
    assert_eq!(owner(&fixture), previous);
    observer.send(b"x");
    observer.until("requires the session controller");
    observer.detach();
    let mut cancel = Peer::ordinary(&fixture, &[], Size::new(100, 24).unwrap());
    cancel.until("[t] Take");
    cancel.send(b"q");
    cancel.until("RESTORED:0");
    assert_eq!(owner(&fixture), previous);
    old.detach();
}

#[test]
fn competition() {
    let fixture = Fixture::cold();
    let mut old = Peer::ordinary(&fixture, &[], Size::new(80, 24).unwrap());
    old.until("control");
    let mut first = Peer::ordinary(&fixture, &[], Size::new(100, 24).unwrap());
    first.until("[t] Take");
    let mut second = Peer::ordinary(&fixture, &[], Size::new(100, 24).unwrap());
    second.until("[t] Take");
    first.send(b"t");
    first.until("control");
    old.until("RESTORED:0");
    let current = owner(&fixture);
    second.send(b"t");
    second.until("RESTORED:2");
    assert_eq!(owner(&fixture), current);
    first.send(b"printf '\\127\\111\\116\\116\\105\\122_OK\\n'\r");
    first.until("WINNER_OK");
    first.detach();
}

#[test]
fn connect() {
    let fixture = Fixture::new();
    let mut old = Peer::new(&fixture, SHELL, Size::new(80, 24).unwrap());
    old.until("control");
    let previous = owner(&fixture);
    let mut next = Peer::new(
        &fixture,
        &["connect", "1", "--take"],
        Size::new(100, 24).unwrap(),
    );
    next.until("control");
    old.until("RESTORED:0");
    assert_ne!(owner(&fixture), previous);
    next.send(b"printf '\\103\\117\\116\\116\\105\\103\\124_OK\\n'\r");
    next.until("CONNECT_OK");
    next.detach();
}
