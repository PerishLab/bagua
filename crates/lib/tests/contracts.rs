use protocol::{Command, Failure, Request};

#[test]
fn unavailable() {
    for command in [
        Command::create(80, 24).unwrap(),
        Command::Workspace(workspace::Operation::Read),
        Command::attach(1, 80, 24).unwrap(),
    ] {
        assert_eq!(
            bagua::dispatch(Request::new(command)),
            Err(Failure::Unavailable)
        );
    }
    let request = Request {
        version: 0,
        id: 1,
        command: Command::create(80, 24).unwrap(),
    };
    assert_eq!(bagua::dispatch(request), Err(Failure::Version));
}

#[test]
fn profile() {
    let profile = bagua::dispatch(Request::new(Command::Capabilities)).unwrap();
    assert_eq!(profile.runtime, pty::SUPPORTED);
    assert_eq!(profile.components.len(), 7);
    assert_eq!(
        profile
            .components
            .iter()
            .filter(|c| c.state == protocol::State::Available)
            .count(),
        if pty::SUPPORTED { 6 } else { 1 }
    );
    assert!(
        profile
            .components
            .iter()
            .all(|c| c.state != protocol::State::Unverified)
    );
}
