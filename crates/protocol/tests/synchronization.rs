use protocol::{Cursor, Failure, Update};
use terminal::Revision;
use workspace::{Id, Target};

fn target() -> Target {
    Target::Pane(Id::new(1).unwrap())
}

#[test]
fn baseline() {
    let mut cursor = Cursor::new(target());
    let first = Revision::new(1);
    let next = Revision::new(2);
    assert_eq!(
        cursor.accept(Update {
            target: target(),
            base: Some(first),
            revision: next
        }),
        Err(Failure::Baseline)
    );
    assert_eq!(cursor.revision(), None);
    cursor
        .accept(Update {
            target: target(),
            base: None,
            revision: first,
        })
        .unwrap();
    cursor
        .accept(Update {
            target: target(),
            base: Some(first),
            revision: Revision::new(5),
        })
        .unwrap();
    assert_eq!(cursor.revision(), Some(Revision::new(5)));
    assert_eq!(
        cursor.accept(Update {
            target: target(),
            base: Some(next),
            revision: Revision::new(8)
        }),
        Err(Failure::Baseline)
    );
    assert_eq!(cursor.revision(), Some(Revision::new(5)));
    cursor
        .accept(Update {
            target: target(),
            base: None,
            revision: Revision::new(8),
        })
        .unwrap();
    assert_eq!(cursor.revision(), Some(Revision::new(8)));
}

#[test]
fn stale() {
    let mut cursor = Cursor::new(target());
    let current = Revision::new(4);
    cursor
        .accept(Update {
            target: target(),
            base: None,
            revision: current,
        })
        .unwrap();
    assert_eq!(
        cursor.accept(Update {
            target: target(),
            base: None,
            revision: Revision::new(2)
        }),
        Err(Failure::Stale)
    );
    assert_eq!(
        cursor.accept(Update {
            target: target(),
            base: Some(current),
            revision: current
        }),
        Err(Failure::Stale)
    );
    assert_eq!(cursor.revision(), Some(current));
}

#[test]
fn ownership() {
    let mut cursor = Cursor::new(target());
    let revision = Revision::new(4);
    cursor
        .accept(Update {
            target: target(),
            base: None,
            revision,
        })
        .unwrap();
    let other = Target::Pane(Id::new(2).unwrap());
    assert_eq!(
        cursor.accept(Update {
            target: other,
            base: Some(revision),
            revision: Revision::new(5)
        }),
        Err(Failure::Target)
    );
    assert_eq!(cursor.revision(), Some(revision));
}
