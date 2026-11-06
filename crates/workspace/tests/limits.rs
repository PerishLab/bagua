mod common;

use common::Fixture;
use workspace::{Axis, Failure, Geometry, Id, Operation, State, Target, World};

#[test]
fn sessions() {
    let mut world = World::default();
    for _ in 0..32 {
        world.create(Geometry::new(2, 1).unwrap()).unwrap();
    }
    let before = world.snapshot();
    assert_eq!(
        world.create(Geometry::new(2, 1).unwrap()),
        Err(Failure::Capacity)
    );
    assert_eq!(world.snapshot(), before);
    assert_eq!(
        world.create(Geometry::new(1, 1).unwrap()),
        Err(Failure::Geometry)
    );
}

#[test]
fn connections() {
    let mut fixture = Fixture::new(Geometry::new(81, 25).unwrap());
    fixture.split(fixture.creation.pane, Axis::Columns);
    for id in 1..64 {
        fixture
            .world
            .apply(
                Id::new(id).unwrap(),
                Operation::Join {
                    session: fixture.creation.session,
                    viewport: Geometry::new(80, 24).unwrap(),
                },
            )
            .unwrap();
    }
    let before = fixture.world.snapshot();
    assert_eq!(
        fixture.world.apply(
            Id::new(9999).unwrap(),
            Operation::Join {
                session: fixture.creation.session,
                viewport: Geometry::new(80, 24).unwrap()
            }
        ),
        Err(Failure::Capacity)
    );
    assert_eq!(fixture.world.snapshot(), before);
    fixture
        .apply(Operation::Detach(fixture.creation.session))
        .unwrap();
    assert_eq!(
        fixture
            .world
            .session(fixture.creation.session)
            .unwrap()
            .clients()
            .count(),
        63
    );
}

#[test]
fn depth() {
    let mut fixture = Fixture::new(Geometry::new(2, u16::MAX).unwrap());
    for _ in 0..15 {
        fixture.split(fixture.creation.pane, Axis::Rows);
    }
    let before = fixture.world.snapshot();
    let split = workspace::Split {
        axis: Axis::Rows,
        ratio: workspace::Ratio::default(),
    };
    assert_eq!(
        fixture.apply(Operation::Split {
            pane: fixture.creation.pane,
            split
        }),
        Err(Failure::Capacity)
    );
    assert_eq!(fixture.world.snapshot(), before);
}

#[test]
fn objects() {
    let mut fixture = Fixture::new(Geometry::new(1024, 64).unwrap());
    let first = fixture.creation;
    for session in 0..2 {
        if session == 1 {
            fixture.creation = fixture
                .world
                .create(Geometry::new(1024, 64).unwrap())
                .unwrap();
            fixture
                .apply(Operation::Join {
                    session: fixture.creation.session,
                    viewport: Geometry::new(1024, 64).unwrap(),
                })
                .unwrap();
            fixture
                .apply(Operation::Acquire {
                    session: fixture.creation.session,
                    fixed: None,
                })
                .unwrap();
        }
        for index in 0..32 {
            let pane = if index == 0 {
                fixture.creation.pane
            } else {
                window(&mut fixture)
            };
            fill(&mut fixture, pane);
        }
    }
    let before = fixture.world.snapshot();
    assert_eq!(
        fixture.apply(Operation::Window(first.session)),
        Err(Failure::Capacity)
    );
    assert_eq!(fixture.world.snapshot(), before);
    fixture.world.observe(first.pane, State::Running).unwrap();
    fixture
        .apply(Operation::Close(Target::Pane(first.pane)))
        .unwrap();
    assert!(fixture.world.pane(first.pane).is_none());
}

fn window(fixture: &mut Fixture) -> Id {
    let change = fixture
        .apply(Operation::Window(fixture.creation.session))
        .unwrap();
    let Target::Pane(pane) = change.created[1] else {
        panic!()
    };
    pane
}

fn fill(fixture: &mut Fixture, first: Id) {
    let mut pending = std::collections::VecDeque::from([first]);
    for _ in 0..63 {
        let pane = pending.pop_front().unwrap();
        let next = fixture.split(pane, Axis::Columns);
        pending.push_back(pane);
        pending.push_back(next);
    }
    let before = fixture.world.snapshot();
    let pane = pending.pop_front().unwrap();
    let split = workspace::Split {
        axis: Axis::Columns,
        ratio: workspace::Ratio::default(),
    };
    assert_eq!(
        fixture.apply(Operation::Split { pane, split }),
        Err(Failure::Capacity)
    );
    assert_eq!(fixture.world.snapshot(), before);
}
