use crate::{App, Clipboard, Gesture, Mouse};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use protocol::{
    Failure,
    wire::{Message, Screen},
};
use terminal::{Engine, Extent, Point};
use workspace::{Geometry, Id, World};

fn app(text: &str) -> App {
    let geometry = Geometry::new(256, 64).unwrap();
    let mut world = World::default();
    let created = world.create(geometry).unwrap();
    let mut app = App::new(
        Id::new(100).unwrap(),
        created.session,
        Extent::new(256, 64).unwrap(),
    );
    app.catalog(world.snapshot()).unwrap();
    let mut engine = Engine::new(Extent::new(256, 64).unwrap(), 0).unwrap();
    for chunk in text.as_bytes().chunks(terminal::CHUNK) {
        engine.feed(chunk).unwrap();
    }
    app.accept(Message::Screen {
        pane: created.pane,
        state: Screen::from(&engine.snapshot()),
    })
    .unwrap();
    app
}

fn select(app: &mut App, end: Point) {
    app.select(Id::new(3).unwrap(), Point::default(), false)
        .unwrap();
    app.select(Id::new(3).unwrap(), end, true).unwrap();
}

#[test]
fn unicode() {
    let line = "é".repeat(255);
    let text = vec![line; 60].join("\r\n");
    let mut app = app(&text);
    app.preferences(Mouse::default(), Clipboard::new("osc52", 262_144).unwrap());
    select(&mut app, Point { x: 254, y: 59 });
    app.handle(Gesture::Copy).unwrap();
    let expected = text.replace("\r\n", "\n");
    assert_eq!(app.clipboard, expected);
    assert!(app.clipboard.len() > 4096);
    let bytes = &app.copying().unwrap().0;
    assert_eq!(&bytes[..7], b"\x1b]52;c;");
    assert_eq!(&bytes[bytes.len() - 2..], b"\x1b\\");
    assert_eq!(
        STANDARD.decode(&bytes[7..bytes.len() - 2]).unwrap(),
        expected.as_bytes()
    );
    app.copied();
    assert!(app.copying().is_none());
}

#[test]
fn wrapping() {
    let mut app = app(&"中".repeat(130));
    select(&mut app, Point { x: 3, y: 1 });
    app.handle(Gesture::Copy).unwrap();
    assert_eq!(app.clipboard, "中".repeat(130));
    app.select(Id::new(3).unwrap(), Point { x: 1, y: 0 }, false)
        .unwrap();
    app.handle(Gesture::Copy).unwrap();
    assert_eq!(app.clipboard, "中");
}

#[test]
fn bounds() {
    let mut app = app("hello");
    app.clipboard = "previous".into();
    app.preferences(Mouse::default(), Clipboard::new("local", 4).unwrap());
    select(&mut app, Point { x: 4, y: 0 });
    assert_eq!(app.handle(Gesture::Copy), Err(Failure::Capacity));
    assert_eq!(app.clipboard, "previous");
    app.preferences(Mouse::default(), Clipboard::new("osc52", 10).unwrap());
    for _ in 0..4 {
        select(&mut app, Point { x: 4, y: 0 });
        app.handle(Gesture::Copy).unwrap();
    }
    app.clipboard = "preserved".into();
    select(&mut app, Point { x: 4, y: 0 });
    assert_eq!(app.handle(Gesture::Copy), Err(Failure::Capacity));
    assert_eq!(app.clipboard, "preserved");
    app.preferences(Mouse::default(), Clipboard::default());
    assert!(app.copying().is_some());
    for _ in 0..4 {
        app.copied();
    }
    assert!(app.copying().is_none());
    assert!(Clipboard::new("osc52", 1_048_577).is_err());
    assert!(Clipboard::new("local", 0).is_err());
    assert!(Clipboard::new("shell", 10).is_err());
}
