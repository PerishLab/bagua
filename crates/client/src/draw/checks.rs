use super::*;
use protocol::wire::{Message, Screen};
use std::time::{Duration, Instant, UNIX_EPOCH};
use workspace::{Geometry, Id, World};

fn app() -> App {
    let mut world = World::default();
    let created = world.create(Geometry::new(80, 24).unwrap()).unwrap();
    let mut app = App::new(
        Id::new(100).unwrap(),
        created.session,
        Extent::new(80, 24).unwrap(),
    );
    app.catalog(world.snapshot()).unwrap();
    app
}

#[test]
fn minutes() {
    let mut app = app();
    let now = Instant::now();
    let wall = UNIX_EPOCH + Duration::from_secs(23 * 3600 + 59 * 60);
    app.tick(now, wall);
    let notice = app.paint().unwrap();
    app.tick(now + Duration::from_secs(2), wall + Duration::from_secs(2));
    let first = app.paint().unwrap();
    let expired = first.render(Some(&notice));
    assert_eq!(
        expired
            .windows(4)
            .filter(|bytes| *bytes == b"\x1b[2K")
            .count(),
        1
    );
    assert!(!expired.windows(4).any(|bytes| bytes == b"\x1b[2J"));
    app.tick(
        now + Duration::from_secs(15),
        wall + Duration::from_secs(15),
    );
    assert!(app.paint().unwrap().render(Some(&first)).is_empty());
    app.tick(
        now + Duration::from_secs(60),
        wall + Duration::from_secs(60),
    );
    let bytes = app.paint().unwrap().render(Some(&first));
    assert_eq!(
        bytes
            .windows(4)
            .filter(|bytes| *bytes == b"\x1b[2K")
            .count(),
        1
    );
    assert!(bytes.windows(5).any(|bytes| bytes == b"00:00"));
    assert!(bytes.len() < 200);
}

#[test]
fn pressure() {
    let mut app = app();
    let mut engine = terminal::Engine::new(Extent::new(80, 24).unwrap(), 0).unwrap();
    let mut previous = app.paint().unwrap();
    let mut bytes = 0;
    for index in 0..1000 {
        engine.feed(format!("\r{index:04}").as_bytes()).unwrap();
        app.accept(Message::Screen {
            pane: Id::new(3).unwrap(),
            state: Screen::from(&engine.snapshot()),
        })
        .unwrap();
        let next = app.paint().unwrap();
        let frame = next.render(Some(&previous));
        assert_eq!(
            frame
                .windows(4)
                .filter(|bytes| *bytes == b"\x1b[2K")
                .count(),
            1
        );
        assert!(frame.len() < 200);
        bytes += frame.len();
        previous = next;
    }
    assert!(bytes < 200_000);
    assert!(app.paint().unwrap().render(Some(&previous)).is_empty());
    app.resize(Extent::new(2, 1).unwrap());
    assert!(
        app.paint()
            .unwrap()
            .render(Some(&previous))
            .windows(4)
            .any(|bytes| bytes == b"\x1b[2J")
    );
}
