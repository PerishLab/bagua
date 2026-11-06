use client::View;
use protocol::{Failure, Frame};
use terminal::{Engine, Extent, Revision};
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
    let mut engine = Engine::new(Extent::new(80, 24).unwrap(), 0).unwrap();
    engine.feed(b"A").unwrap();
    engine.feed(b"B").unwrap();
    let revision = engine.revision();
    view.accept(Frame::snapshot(target(), engine.snapshot()))
        .unwrap();
    view.resize(Extent::new(160, 50).unwrap());
    assert_eq!(view.revision(), Some(revision));
    assert_eq!(view.state().unwrap().contents(), "AB");
    engine.feed(b"C").unwrap();
    let mut frame = Frame::snapshot(target(), engine.snapshot());
    frame.update.base = Some(Revision::new(1));
    assert_eq!(view.accept(frame), Err(Failure::Baseline));
    let mut frame = Frame::snapshot(target(), engine.snapshot());
    frame.update.revision = Revision::new(4);
    assert_eq!(view.accept(frame), Err(Failure::Payload));
    let foreign = Frame::snapshot(Target::Pane(Id::new(2).unwrap()), engine.snapshot());
    assert_eq!(view.accept(foreign), Err(Failure::Target));
    assert_eq!(view.state().unwrap().contents(), "AB");
    assert_eq!(view.revision(), Some(revision));
    view.accept(Frame::snapshot(target(), engine.snapshot()))
        .unwrap();
    engine.feed(b"D").unwrap();
    assert_eq!(view.state().unwrap().contents(), "ABC");
}
