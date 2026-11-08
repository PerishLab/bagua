use client::{App, Gesture};
use protocol::{
    Command, Failure, Input,
    wire::{Message, Screen},
};
use terminal::{Engine, Extent, Point};
use workspace::{Geometry, Id, Operation, Target, World};

fn setup() -> (App, World, Engine) {
    let actor = Id::new(100).unwrap();
    let mut world = World::default();
    let geometry = Geometry::new(8, 3).unwrap();
    let created = world.create(geometry).unwrap();
    world
        .apply(
            actor,
            Operation::Join {
                session: created.session,
                viewport: geometry,
            },
        )
        .unwrap();
    world
        .apply(
            actor,
            Operation::Acquire {
                session: created.session,
                fixed: None,
            },
        )
        .unwrap();
    let mut app = App::new(actor, created.session, Extent::new(8, 3).unwrap());
    app.catalog(world.snapshot()).unwrap();
    let engine = Engine::new(Extent::new(8, 3).unwrap(), 20).unwrap();
    (app, world, engine)
}

fn install(app: &mut App, engine: &Engine) {
    app.accept(Message::Screen {
        pane: Id::new(3).unwrap(),
        state: Screen::from(&engine.snapshot()),
    })
    .unwrap();
}

#[test]
fn selection() {
    let (mut app, _, mut engine) = setup();
    engine.feed(b"hello").unwrap();
    install(&mut app, &engine);
    app.select(Id::new(3).unwrap(), Point { x: 0, y: 0 }, false)
        .unwrap();
    app.select(Id::new(3).unwrap(), Point { x: 4, y: 0 }, true)
        .unwrap();
    engine.feed(b"\rNEW").unwrap();
    install(&mut app, &engine);
    assert_eq!(app.screen(Id::new(3).unwrap()).unwrap().cells[0].text, "h");
    app.handle(Gesture::Copy).unwrap();
    assert_eq!(app.screen(Id::new(3).unwrap()).unwrap().cells[0].text, "N");
    assert_eq!(
        app.handle(Gesture::Paste).unwrap(),
        vec![Command::Send {
            pane: Id::new(3).unwrap(),
            input: Input::Paste("hello".into())
        }]
    );
}

#[test]
fn hostile() {
    let (mut app, mut world, engine) = setup();
    install(&mut app, &engine);
    let mut state = Screen::from(&engine.snapshot());
    state.cells[0].text = "\x1b]52;c;clipboard\x07".into();
    assert_eq!(
        app.accept(Message::Screen {
            pane: Id::new(3).unwrap(),
            state
        }),
        Err(Failure::Payload)
    );
    assert_eq!(app.screen(Id::new(3).unwrap()).unwrap().cells[0].text, "");
    world
        .apply(
            Id::new(100).unwrap(),
            Operation::Close(Target::Pane(Id::new(3).unwrap())),
        )
        .unwrap();
    app.catalog(world.snapshot()).unwrap();
    install(&mut app, &engine);
    assert!(app.screen(Id::new(3).unwrap()).is_none());
}

#[test]
fn clipping() {
    let (mut app, world, mut engine) = setup();
    engine.feed("A中B".as_bytes()).unwrap();
    install(&mut app, &engine);
    app.resize(Extent::new(2, 1).unwrap());
    let mut display = Engine::new(Extent::new(2, 2).unwrap(), 0).unwrap();
    for bytes in app.draw().unwrap().chunks(terminal::CHUNK) {
        display.feed(bytes).unwrap();
    }
    assert_eq!(
        display.snapshot().cell(Point { x: 0, y: 0 }).unwrap().text,
        "A"
    );
    assert!(
        display
            .snapshot()
            .cell(Point { x: 1, y: 0 })
            .unwrap()
            .text
            .trim()
            .is_empty()
    );
    app.handle(Gesture::Pan(terminal::Key::Right)).unwrap();
    assert_eq!(
        world.session(Id::new(1).unwrap()).unwrap().geometry(),
        Geometry::new(8, 3).unwrap()
    );
}

#[test]
fn observer() {
    let (mut app, mut world, engine) = setup();
    install(&mut app, &engine);
    world.disconnect(Id::new(100).unwrap()).unwrap();
    app.catalog(world.snapshot()).unwrap();
    assert_eq!(
        app.handle(Gesture::Input(Input::Text("x".into()))),
        Err(Failure::Workspace(workspace::Failure::Observer))
    );
    assert!(matches!(
        app.handle(Gesture::Acquire).unwrap().as_slice(),
        [Command::Workspace(Operation::Acquire { .. })]
    ));
    assert!(!app.controller());
}

fn historical(engine: &Engine, read: &protocol::wire::Read) -> protocol::wire::History {
    let range = engine.history().range();
    let back = if read.back == 0 {
        engine.extent().height()
    } else {
        read.back
    };
    let top = read
        .top
        .unwrap_or_else(|| range.next.saturating_sub(u64::from(back)).max(range.first));
    protocol::wire::History {
        ticket: read.ticket,
        scope: protocol::wire::Scope {
            host: [1; 16],
            pane: Id::new(3).unwrap(),
            epoch: engine.epoch(),
        },
        first: range.first,
        next: range.next,
        top,
        column: read.column,
        bytes: range.bytes,
        evicted: range.evicted,
        screen: Screen::from(
            &engine
                .snapshot()
                .window((range.next - top) as usize, read.column)
                .unwrap(),
        ),
    }
}

fn request(app: &mut App) -> protocol::wire::Read {
    let commands = app.handle(Gesture::History(-1)).unwrap();
    let Command::History { pane, read } = commands[0].clone() else {
        panic!("history request")
    };
    app.bind(9, pane, &read);
    read
}

#[test]
fn browsing() {
    let (mut app, _, mut engine) = setup();
    engine
        .feed(b"zero\r\none\r\ntwo\r\nthree\r\nfour\r\nfive")
        .unwrap();
    install(&mut app, &engine);
    let read = request(&mut app);
    app.accept(Message::History {
        request: 9,
        state: historical(&engine, &read),
    })
    .unwrap();
    let pane = Id::new(3).unwrap();
    assert_eq!(app.screen(pane).unwrap().cells[0].text, "z");
    let paused = app.screen(pane).unwrap().clone();
    for _ in 0..30 {
        engine.feed(b"\r\nNEW").unwrap();
    }
    install(&mut app, &engine);
    assert_eq!(app.screen(pane).unwrap(), &paused);
    app.select(pane, Point { x: 0, y: 0 }, false).unwrap();
    app.select(pane, Point { x: 3, y: 0 }, true).unwrap();
    app.handle(Gesture::Copy).unwrap();
    assert_eq!(app.screen(pane).unwrap(), &paused);
    app.handle(Gesture::Live).unwrap();
    assert!(app.history(pane).is_none());
    assert_eq!(
        app.handle(Gesture::Paste).unwrap(),
        vec![Command::Send {
            pane,
            input: Input::Paste("zero".into())
        }]
    );
}

#[test]
fn cancellation() {
    let (mut app, _, mut engine) = setup();
    engine.feed(b"zero\r\none\r\ntwo\r\nthree").unwrap();
    install(&mut app, &engine);
    let read = request(&mut app);
    let state = historical(&engine, &read);
    app.accept(Message::History {
        request: 10,
        state: state.clone(),
    })
    .unwrap();
    assert!(app.history(Id::new(3).unwrap()).is_none());
    app.handle(Gesture::Live).unwrap();
    app.accept(Message::History { request: 9, state }).unwrap();
    assert!(app.history(Id::new(3).unwrap()).is_none());
    let read = request(&mut app);
    app.accept(Message::Finished {
        request: 9,
        result: Err(Failure::Baseline.into()),
    })
    .unwrap();
    app.accept(Message::History {
        request: 9,
        state: historical(&engine, &read),
    })
    .unwrap();
    assert!(app.history(Id::new(3).unwrap()).is_none());
}
