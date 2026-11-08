use client::{App, Gesture};
use protocol::{
    Command,
    wire::{FRAME, History, Message, Scope, Screen, encode},
};
use std::time::Instant;
use terminal::{Engine, Extent};
use workspace::{Geometry, Id, World};

#[test]
fn frames() {
    for width in [80, 160] {
        let extent = Extent::new(width, 24).unwrap();
        let mut engine = Engine::new(extent, 50_000).unwrap();
        let line = format!("{}\r\n", "x".repeat(usize::from(width)));
        for _ in 0..50_023 {
            engine.feed(line.as_bytes()).unwrap();
        }
        let mut world = World::default();
        let created = world.create(Geometry::new(width, 24).unwrap()).unwrap();
        let mut app = App::new(Id::new(100).unwrap(), created.session, extent);
        app.catalog(world.snapshot()).unwrap();
        let pane = Id::new(3).unwrap();
        let live = Message::Screen {
            pane,
            state: Screen::from(&engine.snapshot()),
        };
        let livebytes = encode(&live, FRAME).unwrap().len();
        app.accept(live).unwrap();
        let commands = app.handle(Gesture::History(-1)).unwrap();
        let Command::History { read, .. } = &commands[0] else {
            panic!("read")
        };
        app.bind(9, pane, read);
        let range = engine.history().range();
        let state = History {
            ticket: read.ticket,
            scope: Scope {
                host: [1; 16],
                pane,
                epoch: engine.epoch(),
            },
            first: range.first,
            next: range.next,
            top: range.next - 24,
            column: 0,
            bytes: range.bytes,
            evicted: range.evicted,
            screen: Screen::from(&engine.snapshot().view(24).unwrap()),
        };
        let message = Message::History { request: 9, state };
        let pagebytes = encode(&message, FRAME).unwrap().len();
        assert!(pagebytes < livebytes + 1024);
        app.accept(message).unwrap();
        let begin = Instant::now();
        let mut drawn = 0;
        for _ in 0..100 {
            drawn = std::hint::black_box(app.draw().unwrap()).len();
        }
        eprintln!(
            "width={width} rows={} live_bytes={livebytes} page_bytes={pagebytes} draw_bytes={drawn} draw100_ms={}",
            engine.history().count(),
            begin.elapsed().as_millis()
        );
        assert!(drawn < usize::from(width) * 24 * 32);
    }
}
