use workspace::{Axis, Failure, Geometry, Id, Operation, Ratio, Split, World};

fn id(value: u64) -> Id {
    Id::new(value).unwrap()
}

fn setup() -> World {
    let mut world = World::default();
    let size = Geometry::new(80, 24).unwrap();
    let created = world.create(size).unwrap();
    for actor in [100, 200, 300] {
        world
            .apply(
                id(actor),
                Operation::Join {
                    session: created.session,
                    viewport: size,
                },
            )
            .unwrap();
    }
    world
        .apply(
            id(100),
            Operation::Acquire {
                session: created.session,
                fixed: None,
            },
        )
        .unwrap();
    world
        .apply(
            id(100),
            Operation::Split {
                pane: created.pane,
                split: Split {
                    axis: Axis::Columns,
                    ratio: Ratio::default(),
                },
            },
        )
        .unwrap();
    world
}

#[test]
fn admission() {
    let mut world = setup();
    world
        .apply(
            id(200),
            Operation::Resize {
                session: id(1),
                geometry: Geometry::new(2, 1).unwrap(),
            },
        )
        .unwrap();
    let before = world.snapshot();
    assert_eq!(
        world.apply(
            id(200),
            Operation::Take {
                session: id(1),
                expected: Some(id(100)),
                fixed: None
            }
        ),
        Err(Failure::Geometry)
    );
    assert_eq!(world.snapshot(), before);
    assert_eq!(
        world.apply(
            id(300),
            Operation::Take {
                session: id(1),
                expected: None,
                fixed: None
            }
        ),
        Err(Failure::Stale)
    );
    assert_eq!(world.snapshot(), before);
    let canvas = world.session(id(1)).unwrap().geometry();
    world
        .apply(
            id(200),
            Operation::Take {
                session: id(1),
                expected: Some(id(100)),
                fixed: Some(canvas),
            },
        )
        .unwrap();
    let session = world.session(id(1)).unwrap();
    assert_eq!(session.controller(), Some(id(200)));
    assert_eq!(session.geometry(), canvas);
    assert!(session.viewport(id(100)).is_none());
    assert!(session.viewport(id(300)).is_some());
    assert_eq!(session.windows().next().unwrap().panes().count(), 2);
    assert_eq!(
        world.apply(id(100), Operation::Focus(id(3))),
        Err(Failure::Observer)
    );
}

#[test]
fn competition() {
    let mut world = setup();
    world
        .apply(
            id(200),
            Operation::Take {
                session: id(1),
                expected: Some(id(100)),
                fixed: None,
            },
        )
        .unwrap();
    let before = world.snapshot();
    assert_eq!(
        world.apply(
            id(300),
            Operation::Take {
                session: id(1),
                expected: Some(id(100)),
                fixed: None
            }
        ),
        Err(Failure::Stale)
    );
    assert_eq!(world.snapshot(), before);
    world
        .apply(
            id(300),
            Operation::Take {
                session: id(1),
                expected: Some(id(200)),
                fixed: None,
            },
        )
        .unwrap();
    let session = world.session(id(1)).unwrap();
    assert_eq!(session.controller(), Some(id(300)));
    assert!(session.viewport(id(200)).is_none());
    assert_eq!(session.clients().count(), 1);
}

#[test]
fn vacant() {
    let mut world = setup();
    world.disconnect(id(100)).unwrap();
    let before = world.snapshot();
    assert_eq!(
        world.apply(
            id(200),
            Operation::Take {
                session: id(1),
                expected: Some(id(100)),
                fixed: None
            }
        ),
        Err(Failure::Stale)
    );
    assert_eq!(world.snapshot(), before);
    world
        .apply(
            id(200),
            Operation::Take {
                session: id(1),
                expected: None,
                fixed: None,
            },
        )
        .unwrap();
    assert_eq!(world.session(id(1)).unwrap().controller(), Some(id(200)));
}
