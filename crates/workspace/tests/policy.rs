mod common;

use common::Fixture;
use workspace::{Axis, Failure, Geometry, Id, Operation, Sizing};

#[test]
fn handoff() {
    let mut fixture = Fixture::new(Geometry::new(81, 25).unwrap());
    fixture.split(fixture.creation.pane, Axis::Columns);
    let session = fixture.creation.session;
    let next = Id::new(2000).unwrap();
    let viewport = Geometry::new(121, 31).unwrap();
    fixture
        .world
        .apply(
            next,
            Operation::Join {
                session,
                viewport: Geometry::new(80, 24).unwrap(),
            },
        )
        .unwrap();
    fixture
        .world
        .apply(
            next,
            Operation::Resize {
                session,
                geometry: viewport,
            },
        )
        .unwrap();
    assert_eq!(
        fixture.world.session(session).unwrap().geometry(),
        Geometry::new(81, 25).unwrap()
    );
    let before = fixture.world.snapshot();
    assert_eq!(
        fixture.world.apply(
            next,
            Operation::Acquire {
                session,
                fixed: None
            }
        ),
        Err(Failure::Busy)
    );
    assert_eq!(fixture.world.snapshot(), before);
    fixture
        .apply(Operation::Transfer { session, next })
        .unwrap();
    let state = fixture.world.session(session).unwrap();
    assert_eq!(state.controller(), Some(next));
    assert_eq!(state.geometry(), viewport);
    assert_eq!(
        fixture.apply(Operation::Focus(fixture.creation.pane)),
        Err(Failure::Observer)
    );
}

#[test]
fn narrow() {
    let geometry = Geometry::new(81, 25).unwrap();
    let mut fixture = Fixture::new(geometry);
    fixture.split(fixture.creation.pane, Axis::Columns);
    let session = fixture.creation.session;
    let next = Id::new(2000).unwrap();
    fixture
        .world
        .apply(
            next,
            Operation::Join {
                session,
                viewport: Geometry::new(3, 1).unwrap(),
            },
        )
        .unwrap();
    let before = fixture.world.snapshot();
    assert_eq!(
        fixture.apply(Operation::Transfer { session, next }),
        Err(Failure::Geometry)
    );
    assert_eq!(fixture.world.snapshot(), before);
    fixture.world.disconnect(fixture.actor).unwrap();
    assert_eq!(
        fixture.world.apply(
            next,
            Operation::Acquire {
                session,
                fixed: None
            }
        ),
        Err(Failure::Geometry)
    );
    fixture
        .world
        .apply(
            next,
            Operation::Acquire {
                session,
                fixed: Some(geometry),
            },
        )
        .unwrap();
    let state = fixture.world.session(session).unwrap();
    assert_eq!(state.controller(), Some(next));
    assert_eq!(state.sizing(), Sizing::Fixed(geometry));
    assert_eq!(state.viewport(next), Geometry::new(3, 1));
}

#[test]
fn fixed() {
    let geometry = Geometry::new(81, 25).unwrap();
    let mut fixture = Fixture::new(geometry);
    fixture.split(fixture.creation.pane, Axis::Columns);
    let session = fixture.creation.session;
    fixture
        .apply(Operation::Size {
            session,
            sizing: Sizing::Fixed(geometry),
        })
        .unwrap();
    fixture
        .apply(Operation::Resize {
            session,
            geometry: Geometry::new(3, 1).unwrap(),
        })
        .unwrap();
    let state = fixture.world.session(session).unwrap();
    assert_eq!(state.geometry(), geometry);
    assert_eq!(state.viewport(fixture.actor), Geometry::new(3, 1));
    let before = fixture.world.snapshot();
    assert_eq!(
        fixture.apply(Operation::Size {
            session,
            sizing: Sizing::Automatic(Geometry::new(3, 1).unwrap())
        }),
        Err(Failure::Geometry)
    );
    assert_eq!(fixture.world.snapshot(), before);
}
