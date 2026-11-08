use client::{App, Gesture};
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use protocol::{
    Command, Failure, Input,
    wire::{Message, Screen},
};
use terminal::{Engine, Extent};
use workspace::{Axis, Geometry, Id, Operation, Ratio, Split, World};

fn setup() -> (App, World) {
    let geometry = Geometry::new(81, 25).unwrap();
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
    let mut app = App::new(actor, created.session, Extent::new(81, 25).unwrap());
    app.catalog(world.snapshot()).unwrap();
    install(&mut app, "hello");
    (app, world)
}

fn install(app: &mut App, text: &str) {
    for place in app.placements().unwrap() {
        let geometry = place.rect.geometry;
        let mut engine =
            Engine::new(Extent::new(geometry.columns(), geometry.rows()).unwrap(), 0).unwrap();
        engine.feed(text.as_bytes()).unwrap();
        let mut state = Screen::from(&engine.snapshot());
        state.revision = app
            .screen(place.pane)
            .map_or(terminal::Revision::new(1), |screen| {
                screen.revision.next().unwrap()
            });
        app.accept(Message::Screen {
            pane: place.pane,
            state,
        })
        .unwrap();
    }
}

fn mouse(kind: MouseEventKind, x: u16, y: u16, shift: bool) -> MouseEvent {
    MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: if shift {
            KeyModifiers::SHIFT
        } else {
            KeyModifiers::NONE
        },
    }
}

fn select(app: &mut App, shift: bool) {
    app.mouse(mouse(MouseEventKind::Down(MouseButton::Left), 0, 0, shift))
        .unwrap();
    app.mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 4, 0, shift))
        .unwrap();
    app.mouse(mouse(MouseEventKind::Up(MouseButton::Left), 4, 0, shift))
        .unwrap();
}

#[test]
fn release() {
    let (mut app, _) = setup();
    select(&mut app, false);
    assert!(app.copying().is_none());
    assert_eq!(
        app.handle(Gesture::Paste).unwrap(),
        vec![Command::Send {
            pane: Id::new(3).unwrap(),
            input: Input::Paste("hello".into())
        }]
    );
    assert_eq!(
        app.handle(Gesture::Input(Input::Text("x".into()))).unwrap(),
        vec![Command::Send {
            pane: Id::new(3).unwrap(),
            input: Input::Text("x".into())
        }]
    );
    app.mouse(mouse(MouseEventKind::Down(MouseButton::Left), 0, 0, false))
        .unwrap();
    app.mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 4, 0, false))
        .unwrap();
    app.mouse(mouse(MouseEventKind::Up(MouseButton::Left), 81, 0, false))
        .unwrap();
    assert!(!app.pointing());
    assert!(app.handle(Gesture::Copy).is_err());
}

#[test]
fn resize() {
    let (mut app, mut world) = setup();
    world
        .apply(
            app.actor(),
            Operation::Split {
                pane: Id::new(3).unwrap(),
                split: Split {
                    axis: Axis::Columns,
                    ratio: Ratio::default(),
                },
            },
        )
        .unwrap();
    app.catalog(world.snapshot()).unwrap();
    install(&mut app, "hello");
    let before = world.snapshot();
    assert!(
        app.mouse(mouse(MouseEventKind::Down(MouseButton::Left), 40, 0, false))
            .unwrap()
            .is_empty()
    );
    assert!(
        app.mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 50, 0, false))
            .unwrap()
            .is_empty()
    );
    assert_eq!(world.snapshot(), before);
    let commands = app
        .mouse(mouse(MouseEventKind::Up(MouseButton::Left), 50, 0, false))
        .unwrap();
    let Command::Workspace(operation @ Operation::Adjust { .. }) = commands[0].clone() else {
        panic!("adjust required")
    };
    world.apply(app.actor(), operation).unwrap();
    app.catalog(world.snapshot()).unwrap();
    assert_eq!(app.placements().unwrap()[0].rect.geometry.columns(), 50);
    world.disconnect(app.actor()).unwrap();
    app.catalog(world.snapshot()).unwrap();
    assert_eq!(
        app.mouse(mouse(MouseEventKind::Down(MouseButton::Left), 50, 0, false)),
        Err(Failure::Workspace(workspace::Failure::Observer))
    );
}

#[test]
fn tracking() {
    let (mut app, mut world) = setup();
    install(&mut app, "\x1b[?1000h\x1b[?1006hhello");
    let commands = app
        .mouse(mouse(MouseEventKind::Down(MouseButton::Left), 0, 0, false))
        .unwrap();
    assert!(matches!(
        commands.as_slice(),
        [Command::Send {
            input: Input::Mouse(_),
            ..
        }]
    ));
    let commands = app
        .mouse(mouse(MouseEventKind::Up(MouseButton::Left), 4, 0, false))
        .unwrap();
    assert!(matches!(
        commands.as_slice(),
        [Command::Send {
            input: Input::Mouse(_),
            ..
        }]
    ));
    select(&mut app, true);
    assert!(
        matches!(app.handle(Gesture::Paste).unwrap().as_slice(), [Command::Send { input: Input::Paste(text), .. }] if text=="hello")
    );
    assert!(matches!(
        app.mouse(mouse(MouseEventKind::ScrollUp, 0, 0, false))
            .unwrap()
            .as_slice(),
        [Command::Send {
            input: Input::Mouse(_),
            ..
        }]
    ));
    assert!(matches!(
        app.mouse(mouse(MouseEventKind::ScrollUp, 0, 0, true))
            .unwrap()
            .as_slice(),
        [Command::History { .. }]
    ));
    world.disconnect(app.actor()).unwrap();
    app.catalog(world.snapshot()).unwrap();
    select(&mut app, true);
    assert!(app.handle(Gesture::Paste).is_err());
}

#[test]
fn policy() {
    let (mut app, mut world) = setup();
    app.preferences(
        client::Mouse {
            release: false,
            ..Default::default()
        },
        client::Clipboard::default(),
    );
    select(&mut app, false);
    assert!(app.handle(Gesture::Input(Input::Text("x".into()))).is_err());
    app.handle(Gesture::Copy).unwrap();
    app.preferences(
        client::Mouse {
            enabled: false,
            ..Default::default()
        },
        client::Clipboard::default(),
    );
    assert!(
        app.mouse(mouse(MouseEventKind::Down(MouseButton::Left), 0, 0, false))
            .unwrap()
            .is_empty()
    );
    assert!(!app.pointing());
    app.preferences(client::Mouse::default(), client::Clipboard::default());
    app.mouse(mouse(MouseEventKind::Down(MouseButton::Left), 0, 0, false))
        .unwrap();
    app.mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 4, 0, false))
        .unwrap();
    world
        .apply(
            app.actor(),
            Operation::Close(workspace::Target::Pane(Id::new(3).unwrap())),
        )
        .unwrap();
    app.catalog(world.snapshot()).unwrap();
    assert!(!app.pointing());
    assert!(app.handle(Gesture::Copy).is_err());
}
