use client::View;
use protocol::{Failure, Update};
use terminal::{Extent, Revision};
use workspace::{Id, Target};

fn target() -> Target {
    Target::Pane(Id::new(1).unwrap())
}

#[test]
fn independent() {
    let content = Extent::new(213, 50).unwrap();
    let small = View::new(target(), Extent::new(80, 24).unwrap());
    let large = View::new(target(), Extent::new(240, 60).unwrap());
    assert_eq!(small.crop(content, 107, 0), Extent::new(80, 24));
    assert_eq!(large.crop(content, 107, 0), Extent::new(106, 50));
    assert_eq!(small.crop(content, 214, 0), None);
    assert_eq!(content, Extent::new(213, 50).unwrap());
}

#[test]
fn synchronization() {
    let mut view = View::new(target(), Extent::new(80, 24).unwrap());
    let revision = Revision::new(2);
    view.accept(Update {
        target: target(),
        base: None,
        revision,
    })
    .unwrap();
    view.resize(Extent::new(160, 50).unwrap());
    assert_eq!(view.revision(), Some(revision));
    assert_eq!(
        view.accept(Update {
            target: target(),
            base: Some(Revision::new(1)),
            revision: Revision::new(3)
        }),
        Err(Failure::Baseline)
    );
    assert_eq!(view.revision(), Some(revision));
}
