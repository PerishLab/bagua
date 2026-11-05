use bagua::Host;
use protocol::{Command, Failure, Request};
use workspace::{Geometry, Id, Session};

#[test]
fn detach() {
    let controller = Id::new(2).unwrap();
    let mut session = Session::new(Id::new(1).unwrap(), Geometry::new(80, 24).unwrap());
    session.acquire(controller).unwrap();
    let mut host = Host::new(session);
    host.detach(controller);
    assert_eq!(host.session().controller(), None);
    assert_eq!(host.session().id(), Id::new(1).unwrap());
    assert_eq!(host.process().state(), pty::State::Absent);
    assert_eq!(host.revision(), terminal::Revision::default());
}

#[test]
fn unavailable() {
    for command in [
        Command::Create,
        Command::Sessions,
        Command::Attach {
            session: Id::new(1).unwrap(),
        },
    ] {
        assert_eq!(
            bagua::dispatch(Request::new(command)),
            Err(Failure::Unavailable)
        );
    }
    let request = Request {
        version: 0,
        command: Command::Create,
    };
    assert_eq!(bagua::dispatch(request), Err(Failure::Version));
}

#[test]
fn profile() {
    let profile = bagua::dispatch(Request::new(Command::Capabilities)).unwrap();
    assert!(!profile.runtime);
    assert_eq!(profile.components.len(), 7);
    assert_eq!(
        profile
            .components
            .iter()
            .filter(|c| c.state == protocol::State::Available)
            .count(),
        1
    );
    assert!(
        profile
            .components
            .iter()
            .all(|c| c.state != protocol::State::Unverified)
    );
}
