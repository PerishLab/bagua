mod common;

use common::Fixture;
use workspace::{Failure, Geometry, Id, Operation, State, Target};

#[test]
fn identities() {
    let mut fixture = Fixture::new(Geometry::new(81, 25).unwrap());
    let creation = fixture.creation;
    assert_eq!(
        fixture.world.location(Target::Pane(creation.pane)),
        Ok((creation.session, Some(creation.window)))
    );
    assert_eq!(
        fixture.world.location(Target::Window(creation.pane)),
        Err(Failure::Missing)
    );
    let other = fixture
        .world
        .create(Geometry::new(20, 10).unwrap())
        .unwrap();
    assert!(other.session > creation.pane);
    assert!(other.pane > other.window);
    let before = fixture.world.snapshot();
    assert_eq!(
        fixture.apply(Operation::Close(Target::Pane(other.pane))),
        Err(Failure::Observer)
    );
    assert_eq!(fixture.world.snapshot(), before);
    assert_eq!(
        fixture.world.pane(creation.pane).unwrap().state(),
        State::Pending
    );
}

#[test]
fn exit() {
    let mut fixture = Fixture::new(Geometry::new(81, 25).unwrap());
    let pane = fixture.creation.pane;
    assert_eq!(
        fixture.world.input(fixture.actor, pane),
        Err(Failure::State)
    );
    let before = fixture.world.snapshot();
    assert_eq!(fixture.world.finish(pane), Err(Failure::State));
    assert_eq!(
        fixture.world.observe(pane, State::Exited(Some(0))),
        Err(Failure::State)
    );
    assert_eq!(fixture.world.snapshot(), before);
    fixture.world.observe(pane, State::Running).unwrap();
    fixture.world.input(fixture.actor, pane).unwrap();
    assert_eq!(fixture.world.finish(pane), Err(Failure::State));
    fixture.world.observe(pane, State::Exited(Some(7))).unwrap();
    assert_eq!(
        fixture.world.pane(pane).unwrap().state(),
        State::Exited(Some(7))
    );
    assert_eq!(
        fixture.world.input(fixture.actor, pane),
        Err(Failure::State)
    );
    assert_eq!(
        fixture.world.observe(pane, State::Running),
        Err(Failure::State)
    );
    let change = fixture.world.finish(pane).unwrap();
    assert_eq!(change.retired, vec![pane]);
    assert!(fixture.world.pane(pane).is_none());
    assert!(fixture.world.session(fixture.creation.session).is_none());
    assert_eq!(
        fixture.world.observe(pane, State::Exited(Some(0))),
        Err(Failure::Missing)
    );
    assert_eq!(fixture.world.finish(pane), Err(Failure::Missing));
}

#[test]
fn retirement() {
    let mut fixture = Fixture::new(Geometry::new(81, 25).unwrap());
    let first = fixture.creation.pane;
    let second = fixture.split(first, workspace::Axis::Columns);
    let old = fixture.world.snapshot();
    let change = fixture
        .apply(Operation::Close(Target::Window(fixture.creation.window)))
        .unwrap();
    assert_eq!(change.retired, vec![first, second]);
    assert_eq!(
        old.sessions()
            .next()
            .unwrap()
            .windows()
            .next()
            .unwrap()
            .panes()
            .count(),
        2
    );
    assert!(fixture.world.session(fixture.creation.session).is_none());
    let created = fixture
        .world
        .create(Geometry::new(81, 25).unwrap())
        .unwrap();
    assert!(created.session > second);
    assert!(created.pane > created.window);
}

#[test]
fn disconnect() {
    let mut fixture = Fixture::new(Geometry::new(81, 25).unwrap());
    fixture
        .world
        .observe(fixture.creation.pane, State::Running)
        .unwrap();
    let observer = Id::new(2000).unwrap();
    fixture
        .world
        .apply(
            observer,
            Operation::Join {
                session: fixture.creation.session,
                viewport: Geometry::new(10, 3).unwrap(),
            },
        )
        .unwrap();
    assert_eq!(
        fixture.world.input(observer, fixture.creation.pane),
        Err(Failure::Observer)
    );
    fixture.world.disconnect(fixture.actor).unwrap();
    let session = fixture.world.session(fixture.creation.session).unwrap();
    assert_eq!(session.controller(), None);
    assert_eq!(session.clients().count(), 1);
    assert_eq!(session.geometry(), Geometry::new(81, 25).unwrap());
    assert_eq!(
        fixture.world.pane(fixture.creation.pane).unwrap().state(),
        State::Running
    );
    let revision = fixture.world.revision();
    assert!(fixture.world.disconnect(fixture.actor).unwrap().is_empty());
    assert_eq!(fixture.world.revision(), revision);
}

#[test]
fn failure() {
    let mut fixture = Fixture::new(Geometry::new(81, 25).unwrap());
    fixture
        .world
        .observe(fixture.creation.pane, State::Failed)
        .unwrap();
    assert_eq!(
        fixture.world.input(fixture.actor, fixture.creation.pane),
        Err(Failure::State)
    );
    assert_eq!(
        fixture.world.observe(fixture.creation.pane, State::Running),
        Err(Failure::State)
    );
}
