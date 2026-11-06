use protocol::{Failure, Frame, Latest, Packet, Replica, Stamped, Update};
use terminal::Revision;
use workspace::{Id, Target};

#[derive(Debug, PartialEq, Eq)]
struct State {
    revision: Revision,
    values: Vec<u8>,
}

impl Stamped for State {
    fn revision(&self) -> Revision {
        self.revision
    }
}

fn target() -> Target {
    Target::Pane(Id::new(1).unwrap())
}

fn merge(state: &State, values: Vec<u8>) -> Result<State, Failure> {
    if values.is_empty() {
        return Err(Failure::Payload);
    }
    Ok(State {
        revision: Revision::new(3),
        values: state.values.iter().copied().chain(values).collect(),
    })
}

#[test]
fn installation() {
    let mut replica = Replica::new(target());
    let state = State {
        revision: Revision::new(1),
        values: vec![1],
    };
    replica
        .apply(Packet::Snapshot(Frame::snapshot(target(), state)), merge)
        .unwrap();
    let update = Update {
        target: target(),
        base: Some(Revision::new(1)),
        revision: Revision::new(3),
    };
    assert_eq!(
        replica.apply(
            Packet::Delta {
                update,
                change: Vec::new()
            },
            merge
        ),
        Err(Failure::Payload)
    );
    assert_eq!(replica.revision(), Some(Revision::new(1)));
    assert_eq!(replica.state().unwrap().values, vec![1]);
    let mismatched = Update {
        revision: Revision::new(4),
        ..update
    };
    assert_eq!(
        replica.apply(
            Packet::Delta {
                update: mismatched,
                change: vec![4]
            },
            merge
        ),
        Err(Failure::Payload)
    );
    assert_eq!(replica.revision(), Some(Revision::new(1)));
    replica
        .apply(
            Packet::Delta {
                update,
                change: vec![2, 3],
            },
            merge,
        )
        .unwrap();
    assert_eq!(replica.state().unwrap().values, vec![1, 2, 3]);
    assert_eq!(replica.revision(), Some(Revision::new(3)));
}

#[test]
fn mismatch() {
    let mut replica = Replica::new(target());
    let state = State {
        revision: Revision::new(1),
        values: vec![1],
    };
    let mut frame = Frame::snapshot(target(), state);
    frame.update.revision = Revision::new(2);
    assert_eq!(
        replica.apply(Packet::Snapshot(frame), merge),
        Err(Failure::Payload)
    );
    assert!(replica.state().is_none());
    assert_eq!(replica.revision(), None);
    let update = Update {
        target: target(),
        base: Some(Revision::new(1)),
        revision: Revision::new(3),
    };
    assert_eq!(
        replica.apply(
            Packet::Delta {
                update,
                change: vec![1]
            },
            merge
        ),
        Err(Failure::Baseline)
    );
}

#[test]
fn gap() {
    let mut replica = Replica::new(target());
    replica
        .apply(
            Packet::Snapshot(Frame::snapshot(
                target(),
                State {
                    revision: Revision::new(1),
                    values: vec![1],
                },
            )),
            merge,
        )
        .unwrap();
    let update = Update {
        target: target(),
        base: Some(Revision::new(2)),
        revision: Revision::new(3),
    };
    assert_eq!(
        replica.apply(
            Packet::Delta {
                update,
                change: vec![2]
            },
            merge
        ),
        Err(Failure::Baseline)
    );
    assert_eq!(replica.revision(), Some(Revision::new(1)));
    replica
        .apply(
            Packet::Snapshot(Frame::snapshot(
                target(),
                State {
                    revision: Revision::new(8),
                    values: vec![8],
                },
            )),
            merge,
        )
        .unwrap();
    assert_eq!(replica.state().unwrap().values, vec![8]);
}

#[test]
fn foreign() {
    let mut replica = Replica::new(target());
    let foreign = Target::Pane(Id::new(2).unwrap());
    let frame = Frame::snapshot(
        foreign,
        State {
            revision: Revision::new(1),
            values: vec![1],
        },
    );
    assert_eq!(
        replica.apply(Packet::Snapshot(frame), merge),
        Err(Failure::Target)
    );
    assert!(replica.state().is_none());
}

#[test]
fn coalescing() {
    let mut latest = Latest::new(target());
    latest
        .offer(State {
            revision: Revision::new(1),
            values: vec![1],
        })
        .unwrap();
    latest
        .offer(State {
            revision: Revision::new(4),
            values: vec![4],
        })
        .unwrap();
    assert_eq!(
        latest.offer(State {
            revision: Revision::new(3),
            values: vec![3]
        }),
        Err(Failure::Stale)
    );
    let frame = latest.take().unwrap();
    assert_eq!(frame.state.values, vec![4]);
    assert_eq!(frame.update.base, None);
    assert!(latest.take().is_none());
}

#[test]
fn owner() {
    let mut world = workspace::World::default();
    let created = world
        .create(workspace::Geometry::new(80, 24).unwrap())
        .unwrap();
    let frame = Frame::snapshot(Target::Session(created.session), world.snapshot());
    let mut replica = Replica::new(Target::Session(created.session));
    replica
        .apply(Packet::<_, ()>::Snapshot(frame), |_, ()| {
            Err(Failure::Payload)
        })
        .unwrap();
    assert_eq!(
        replica.revision().unwrap().value(),
        replica.state().unwrap().revision()
    );
}
