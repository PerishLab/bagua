mod common;

use common::Fixture;
use std::collections::BTreeSet;
use workspace::{Axis, Failure, Geometry, Operation, Target};

#[test]
fn partition() {
    let geometry = Geometry::new(81, 25).unwrap();
    let mut fixture = Fixture::new(geometry);
    let second = fixture.split(fixture.creation.pane, Axis::Columns);
    let third = fixture.split(second, Axis::Rows);
    let session = fixture.world.session(fixture.creation.session).unwrap();
    let window = session.windows().next().unwrap();
    let placements = window.placements(geometry).unwrap();
    assert_eq!(placements.len(), 3);
    assert_eq!(window.focus(), third);
    let mut cells = BTreeSet::new();
    for placement in placements {
        let rect = placement.rect;
        for y in rect.y..rect.y + rect.geometry.rows() {
            for x in rect.x..rect.x + rect.geometry.columns() {
                assert!(x < geometry.columns() && y < geometry.rows());
                assert!(cells.insert((x, y)));
            }
        }
    }
    assert_eq!(cells.len(), 1960);
}

#[test]
fn collapse() {
    let geometry = Geometry::new(81, 25).unwrap();
    let mut fixture = Fixture::new(geometry);
    let second = fixture.split(fixture.creation.pane, Axis::Columns);
    fixture
        .apply(Operation::Close(Target::Pane(fixture.creation.pane)))
        .unwrap();
    let session = fixture.world.session(fixture.creation.session).unwrap();
    let window = session.windows().next().unwrap();
    assert_eq!(window.focus(), second);
    let placements = window.placements(geometry).unwrap();
    assert_eq!(placements.len(), 1);
    assert_eq!(placements[0].rect.geometry, geometry);
}

#[test]
fn rollback() {
    let mut fixture = Fixture::new(Geometry::new(5, 3).unwrap());
    let second = fixture.split(fixture.creation.pane, Axis::Columns);
    let before = fixture.world.snapshot();
    let split = workspace::Split {
        axis: Axis::Columns,
        ratio: workspace::Ratio::default(),
    };
    assert_eq!(
        fixture.apply(Operation::Split {
            pane: fixture.creation.pane,
            split
        }),
        Err(Failure::Geometry)
    );
    assert_eq!(fixture.world.snapshot(), before);
    assert_eq!(
        fixture.apply(Operation::Resize {
            session: fixture.creation.session,
            geometry: Geometry::new(4, 3).unwrap()
        }),
        Err(Failure::Geometry)
    );
    assert_eq!(fixture.world.snapshot(), before);
    assert!(workspace::Ratio::new(0).is_none());
    assert!(workspace::Ratio::new(1000).is_none());
    fixture
        .apply(Operation::Resize {
            session: fixture.creation.session,
            geometry: Geometry::new(11, 3).unwrap(),
        })
        .unwrap();
    let next = fixture.split(fixture.creation.pane, Axis::Columns);
    assert_eq!(next.value(), second.value() + 1);
}

#[test]
fn focus() {
    let mut fixture = Fixture::new(Geometry::new(81, 25).unwrap());
    let first = fixture.creation.pane;
    fixture.split(first, Axis::Columns);
    fixture
        .apply(Operation::Window(fixture.creation.session))
        .unwrap();
    fixture.apply(Operation::Focus(first)).unwrap();
    let session = fixture.world.session(fixture.creation.session).unwrap();
    assert_eq!(session.selected(), Some(fixture.creation.window));
    assert_eq!(session.windows().next().unwrap().focus(), first);
}

#[test]
fn dividers() {
    let geometry = Geometry::new(81, 25).unwrap();
    let mut fixture = Fixture::new(geometry);
    let second = fixture.split(fixture.creation.pane, Axis::Columns);
    fixture.split(second, Axis::Rows);
    let window = fixture
        .world
        .session(fixture.creation.session)
        .unwrap()
        .windows()
        .next()
        .unwrap();
    let dividers = window.dividers(geometry).unwrap();
    assert_eq!(dividers.len(), 2);
    let divider = dividers[0];
    assert!(divider.contains(40, 0));
    assert!(divider.contains(40, 24));
    assert!(!divider.contains(39, 0));
    let ratio = divider.ratio(50).unwrap();
    let operation = Operation::Adjust {
        window: fixture.creation.window,
        divider,
        ratio,
    };
    let before = fixture.world.snapshot();
    assert_eq!(
        fixture
            .world
            .apply(workspace::Id::new(1001).unwrap(), operation.clone()),
        Err(Failure::Observer)
    );
    assert_eq!(fixture.world.snapshot(), before);
    fixture.apply(operation.clone()).unwrap();
    let window = fixture
        .world
        .session(fixture.creation.session)
        .unwrap()
        .windows()
        .next()
        .unwrap();
    let places = window.placements(geometry).unwrap();
    assert_eq!(places[0].rect.geometry.columns(), 50);
    assert_eq!(places[1].rect.x, 51);
    let before = fixture.world.snapshot();
    assert_eq!(fixture.apply(operation), Err(Failure::State));
    assert_eq!(fixture.world.snapshot(), before);
    let divider = fixture
        .world
        .session(fixture.creation.session)
        .unwrap()
        .windows()
        .next()
        .unwrap()
        .dividers(geometry)
        .unwrap()[0];
    assert_eq!(
        fixture.apply(Operation::Adjust {
            window: fixture.creation.window,
            divider,
            ratio: workspace::Ratio::new(1).unwrap()
        }),
        Err(Failure::Geometry)
    );
    assert_eq!(fixture.world.snapshot(), before);
    fixture
        .apply(Operation::Close(Target::Pane(second)))
        .unwrap();
    assert_eq!(
        fixture.apply(Operation::Adjust {
            window: fixture.creation.window,
            divider: dividers[1],
            ratio
        }),
        Err(Failure::State)
    );
}
#[test]
fn headers() {
    let geometry = Geometry::new(81, 5).unwrap();
    let mut fixture = Fixture::new(geometry);
    fixture.split(fixture.creation.pane, Axis::Rows);
    fixture
        .apply(Operation::Header {
            session: fixture.creation.session,
            enabled: true,
        })
        .unwrap();
    let session = fixture.world.session(fixture.creation.session).unwrap();
    assert!(session.header());
    let places = session
        .windows()
        .next()
        .unwrap()
        .placements(session.canvas())
        .unwrap();
    for place in places {
        assert_eq!(place.rect.y, place.frame.y + 1);
        assert_eq!(place.rect.geometry.rows(), 1);
        assert_eq!(place.frame.geometry.rows(), 2);
    }
    let before = fixture.world.snapshot();
    assert_eq!(
        fixture.apply(Operation::Resize {
            session: fixture.creation.session,
            geometry: Geometry::new(81, 4).unwrap()
        }),
        Err(Failure::Geometry)
    );
    assert_eq!(fixture.world.snapshot(), before);
    assert_eq!(
        fixture.world.apply(
            workspace::Id::new(999).unwrap(),
            Operation::Header {
                session: fixture.creation.session,
                enabled: false
            }
        ),
        Err(Failure::Observer)
    );
    assert_eq!(fixture.world.snapshot(), before);
    fixture
        .apply(Operation::Header {
            session: fixture.creation.session,
            enabled: false,
        })
        .unwrap();
    assert!(
        !fixture
            .world
            .session(fixture.creation.session)
            .unwrap()
            .header()
    );
}

#[test]
fn empty() {
    let mut fixture = Fixture::new(Geometry::new(2, 1).unwrap());
    let before = fixture.world.snapshot();
    assert_eq!(
        fixture.apply(Operation::Header {
            session: fixture.creation.session,
            enabled: true
        }),
        Err(Failure::Geometry)
    );
    assert_eq!(fixture.world.snapshot(), before);
    fixture
        .apply(Operation::Close(Target::Pane(fixture.creation.pane)))
        .unwrap();
    fixture
        .apply(Operation::Header {
            session: fixture.creation.session,
            enabled: true,
        })
        .unwrap();
    let before = fixture.world.snapshot();
    assert_eq!(
        fixture.apply(Operation::Window(fixture.creation.session)),
        Err(Failure::Geometry)
    );
    assert_eq!(fixture.world.snapshot(), before);
}
