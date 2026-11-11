use client::{App, Clipboard, Gesture, Mouse};
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use protocol::{
    Command, Input,
    wire::{Message, Screen},
};
use terminal::{Engine, Extent, Tracking};
use workspace::{Axis, Geometry, Id, Operation, Ratio, Split, State, World};

fn setup() -> (App, World) {
    let geometry = Geometry::new(80, 24).unwrap();
    let mut world = World::default();
    let created = world.create(geometry).unwrap();
    let actor = Id::new(100).unwrap();
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
    world.observe(created.pane, State::Running).unwrap();
    let mut app = App::new(actor, created.session, Extent::new(80, 24).unwrap());
    app.catalog(world.snapshot()).unwrap();
    install(&mut app, created.pane, b"hello");
    (app, world)
}

fn install(app: &mut App, pane: Id, bytes: &[u8]) {
    let place = app
        .placements()
        .unwrap()
        .into_iter()
        .find(|place| place.pane == pane)
        .unwrap();
    let size = place.rect.geometry;
    let mut engine = Engine::new(Extent::new(size.columns(), size.rows()).unwrap(), 0).unwrap();
    engine.feed(bytes).unwrap();
    let mut state = Screen::from(&engine.snapshot());
    state.revision = app.screen(pane).map_or(terminal::Revision::new(1), |old| {
        old.revision.next().unwrap()
    });
    app.accept(Message::Screen { pane, state }).unwrap();
}

fn tracking(app: &App) -> Tracking {
    let mut outer = Engine::new(Extent::new(80, 30).unwrap(), 0).unwrap();
    outer.feed(&app.draw().unwrap()).unwrap();
    outer.modes().tracking
}

fn mouse(kind: MouseEventKind) -> MouseEvent {
    MouseEvent {
        kind,
        column: 0,
        row: 0,
        modifiers: KeyModifiers::SHIFT,
    }
}

#[test]
fn demand() {
    let (mut app, mut world) = setup();
    let pane = app.focus().unwrap();
    assert_eq!(tracking(&app), Tracking::Drag);
    install(&mut app, pane, b"\x1b[?1003h\x1b[?1006hhello");
    assert_eq!(tracking(&app), Tracking::Motion);
    let moved = MouseEvent {
        kind: MouseEventKind::Moved,
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    assert!(matches!(
        app.mouse(moved).unwrap().as_slice(),
        [Command::Send {
            input: Input::Mouse(_),
            ..
        }]
    ));
    for sequence in [b"\x1b[?1003l".as_slice(), b"\x1b[?1000h", b"\x1b[?1002h"] {
        install(&mut app, pane, sequence);
        assert_eq!(tracking(&app), Tracking::Drag);
    }
    install(&mut app, pane, b"\x1b[?1003h");
    app.recovering(true);
    assert_eq!(tracking(&app), Tracking::Drag);
    app.recovering(false);
    assert_eq!(tracking(&app), Tracking::Motion);
    let disabled = Mouse {
        enabled: false,
        ..Mouse::default()
    };
    app.preferences(disabled, Clipboard::default());
    assert_eq!(tracking(&app), Tracking::Off);
    app.preferences(Mouse::default(), Clipboard::default());
    assert_eq!(tracking(&app), Tracking::Motion);
    world.disconnect(app.actor()).unwrap();
    app.catalog(world.snapshot()).unwrap();
    assert_eq!(tracking(&app), Tracking::Drag);
}

#[test]
fn visibility() {
    let (mut app, mut world) = setup();
    let pane = app.focus().unwrap();
    world
        .apply(
            app.actor(),
            Operation::Split {
                pane,
                split: Split {
                    axis: Axis::Columns,
                    ratio: Ratio::new(500).unwrap(),
                },
            },
        )
        .unwrap();
    app.catalog(world.snapshot()).unwrap();
    let other = app.focus().unwrap();
    world.observe(other, State::Running).unwrap();
    app.catalog(world.snapshot()).unwrap();
    install(&mut app, pane, b"\x1b[?1003h");
    install(&mut app, other, b"hello");
    assert_ne!(app.focus(), Some(pane));
    assert_eq!(tracking(&app), Tracking::Motion);
    world
        .apply(app.actor(), Operation::Window(app.target()))
        .unwrap();
    app.catalog(world.snapshot()).unwrap();
    assert_eq!(tracking(&app), Tracking::Drag);
    world.apply(app.actor(), Operation::Focus(pane)).unwrap();
    app.catalog(world.snapshot()).unwrap();
    assert_eq!(tracking(&app), Tracking::Motion);
    world
        .apply(app.actor(), Operation::Close(workspace::Target::Pane(pane)))
        .unwrap();
    app.catalog(world.snapshot()).unwrap();
    assert_eq!(tracking(&app), Tracking::Drag);
}

#[test]
fn capture() {
    let (mut app, _) = setup();
    let pane = app.focus().unwrap();
    install(&mut app, pane, b"\x1b[?1003h\x1b[?1006hhello");
    assert_eq!(tracking(&app), Tracking::Motion);
    app.mouse(mouse(MouseEventKind::Down(MouseButton::Left)))
        .unwrap();
    assert_eq!(tracking(&app), Tracking::Drag);
    app.mouse(MouseEvent {
        column: 4,
        ..mouse(MouseEventKind::Drag(MouseButton::Left))
    })
    .unwrap();
    app.mouse(MouseEvent {
        column: 4,
        ..mouse(MouseEventKind::Up(MouseButton::Left))
    })
    .unwrap();
    assert_eq!(tracking(&app), Tracking::Motion);
    app.preferences(
        Mouse {
            release: false,
            ..Mouse::default()
        },
        Clipboard::default(),
    );
    app.mouse(mouse(MouseEventKind::Down(MouseButton::Left)))
        .unwrap();
    app.mouse(MouseEvent {
        column: 4,
        ..mouse(MouseEventKind::Drag(MouseButton::Left))
    })
    .unwrap();
    app.mouse(MouseEvent {
        column: 4,
        ..mouse(MouseEventKind::Up(MouseButton::Left))
    })
    .unwrap();
    assert_eq!(tracking(&app), Tracking::Drag);
    app.handle(Gesture::Input(Input::Key(terminal::Key::Escape)))
        .unwrap();
    assert_eq!(tracking(&app), Tracking::Motion);
}

#[test]
fn history() {
    let (mut app, _) = setup();
    let pane = app.focus().unwrap();
    install(&mut app, pane, b"\x1b[?1003hhello");
    let commands = app.handle(Gesture::History(-1)).unwrap();
    let Command::History { read, .. } = &commands[0] else {
        panic!("history")
    };
    app.bind(9, pane, read);
    let mut screen = app.screen(pane).unwrap().clone();
    screen.cursor.visible = false;
    let state = protocol::wire::History {
        ticket: read.ticket,
        scope: protocol::wire::Scope {
            host: [1; 16],
            pane,
            epoch: 1,
        },
        first: 0,
        next: 24,
        top: 0,
        column: 0,
        bytes: 0,
        evicted: 0,
        screen,
    };
    app.accept(Message::History { request: 9, state }).unwrap();
    assert!(app.history(pane).is_some());
    assert_eq!(tracking(&app), Tracking::Drag);
    app.handle(Gesture::Input(Input::Key(terminal::Key::End)))
        .unwrap();
    assert_eq!(tracking(&app), Tracking::Motion);
}
