use workspace::{Entry, Failure, Geometry, Id, Operation, Outcome, Target, World};

fn id(value: u64) -> Id {
    Id::new(value).unwrap()
}

fn entry(name: &str, observe: bool) -> Operation {
    Operation::Enter(Entry {
        name: name.into(),
        viewport: Geometry::new(80, 24).unwrap(),
        create: true,
        observe,
        fixed: false,
    })
}

#[test]
fn acquisition() {
    let mut world = World::default();
    assert!(
        matches!(world.apply(id(100), entry("workspace", false)).unwrap(), Outcome::Entered { session, change } if session == id(1) && change.created.len() == 3)
    );
    let before = world.snapshot();
    assert_eq!(
        world.apply(id(200), entry("workspace", false)),
        Err(Failure::Busy)
    );
    assert_eq!(world.snapshot(), before);
    assert!(
        matches!(world.apply(id(200), entry("workspace", true)).unwrap(), Outcome::Entered { session, change } if session == id(1) && change.created.is_empty())
    );
    assert_eq!(world.find("workspace").unwrap().controller(), Some(id(100)));
    assert_eq!(
        world.find("workspace").unwrap().viewport(id(200)),
        Geometry::new(80, 24)
    );
    let before = world.snapshot();
    assert_eq!(
        world.apply(id(300), entry("absent", true)),
        Err(Failure::Missing)
    );
    assert_eq!(world.snapshot(), before);
}

#[test]
fn names() {
    let mut world = World::default();
    let geometry = Geometry::new(80, 24).unwrap();
    let created = world.named(geometry, "工作台".into()).unwrap();
    let before = world.snapshot();
    assert_eq!(
        world.named(geometry, "工作台".into()),
        Err(Failure::Conflict)
    );
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.find("工作台").unwrap().id(), created.session);
    for name in ["", " padded", "padded ", "bad\nname", &"x".repeat(65)] {
        assert_eq!(world.named(geometry, name.into()), Err(Failure::Name));
        assert_eq!(world.snapshot(), before);
    }
    assert!(world.named(geometry, "Workspace".into()).is_ok());
    assert!(world.named(geometry, "workspace".into()).is_ok());
}

#[test]
fn rename() {
    let mut world = World::default();
    world.apply(id(100), entry("workspace", false)).unwrap();
    world.apply(id(100), Operation::Window(id(1))).unwrap();
    let before = world.snapshot();
    assert_eq!(
        world.apply(
            id(200),
            Operation::Rename {
                target: Target::Session(id(1)),
                name: "other".into()
            }
        ),
        Err(Failure::Observer)
    );
    assert_eq!(world.snapshot(), before);
    assert_eq!(
        world.apply(
            id(100),
            Operation::Rename {
                target: Target::Window(id(4)),
                name: "window-1".into()
            }
        ),
        Err(Failure::Conflict)
    );
    assert_eq!(world.snapshot(), before);
    world
        .apply(
            id(100),
            Operation::Rename {
                target: Target::Window(id(4)),
                name: "editor".into(),
            },
        )
        .unwrap();
    world
        .apply(
            id(100),
            Operation::Rename {
                target: Target::Session(id(1)),
                name: "work".into(),
            },
        )
        .unwrap();
    assert!(world.find("workspace").is_none());
    assert_eq!(world.find("work").unwrap().id(), id(1));
    world
        .apply(id(100), Operation::Close(Target::Window(id(2))))
        .unwrap();
    assert_eq!(
        world.session(id(1)).unwrap().windows().next().unwrap().id(),
        id(4)
    );
    world.apply(id(100), Operation::Window(id(1))).unwrap();
    let ids: Vec<_> = world
        .session(id(1))
        .unwrap()
        .windows()
        .map(|window| window.id())
        .collect();
    assert_eq!(ids, vec![id(4), id(6)]);
}

#[test]
fn fixed() {
    let mut world = World::default();
    world.apply(id(100), entry("workspace", false)).unwrap();
    world
        .apply(
            id(100),
            Operation::Split {
                pane: id(3),
                split: workspace::Split {
                    axis: workspace::Axis::Columns,
                    ratio: workspace::Ratio::default(),
                },
            },
        )
        .unwrap();
    world.disconnect(id(100)).unwrap();
    let before = world.snapshot();
    let narrow = Entry {
        name: "workspace".into(),
        viewport: Geometry::new(2, 1).unwrap(),
        create: false,
        observe: false,
        fixed: false,
    };
    assert_eq!(
        world.apply(id(200), Operation::Enter(narrow.clone())),
        Err(Failure::Geometry)
    );
    assert_eq!(world.snapshot(), before);
    world
        .apply(
            id(200),
            Operation::Enter(Entry {
                fixed: true,
                ..narrow
            }),
        )
        .unwrap();
    assert_eq!(
        world.session(id(1)).unwrap().geometry(),
        Geometry::new(80, 24).unwrap()
    );
    assert_eq!(world.session(id(1)).unwrap().controller(), Some(id(200)));
}
