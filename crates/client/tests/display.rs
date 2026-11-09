use client::{App, Decor};
use protocol::wire::{Context, Message, Screen};
use std::time::{Duration, Instant, UNIX_EPOCH};
use terminal::{Color, Engine, Extent, Point};
use workspace::{Geometry, Id, Operation, State, World};

fn setup() -> (App, World) {
    let geometry = Geometry::new(100, 8).unwrap();
    let actor = Id::new(100).unwrap();
    let mut world = World::default();
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
    world
        .apply(
            actor,
            Operation::Header {
                session: created.session,
                enabled: true,
            },
        )
        .unwrap();
    world.observe(created.pane, State::Running).unwrap();
    let mut app = App::new(actor, created.session, Extent::new(100, 8).unwrap());
    app.catalog(world.snapshot()).unwrap();
    let mut engine = Engine::new(Extent::new(100, 7).unwrap(), 0).unwrap();
    engine.feed(b"content").unwrap();
    app.accept(Message::Screen {
        pane: created.pane,
        state: Screen::from(&engine.snapshot()),
    })
    .unwrap();
    (app, world)
}

fn picture(app: &App) -> terminal::Snapshot {
    let mut engine = Engine::new(Extent::new(100, 20).unwrap(), 0).unwrap();
    for chunk in app.draw().unwrap().chunks(terminal::CHUNK) {
        engine.feed(chunk).unwrap();
    }
    engine.snapshot()
}

fn context() -> Context {
    Context {
        pid: 10,
        session: 10,
        started: 1,
        command: "sh".into(),
        cwd: "/safe/path".into(),
    }
}

#[test]
fn observation() {
    let (mut app, mut world) = setup();
    let pane = Id::new(3).unwrap();
    let now = Instant::now();
    app.tick(now, UNIX_EPOCH);
    assert!(picture(&app).contents().contains("cwd?"));
    assert_eq!(app.needing(), vec![pane]);
    app.context(pane, Some(context()), now).unwrap();
    let screen = picture(&app);
    assert!(screen.contents().contains("cmd~sh"));
    assert!(screen.contents().contains("cwd~/safe/path"));
    assert_eq!(
        screen.cell(Point { x: 0, y: 0 }).unwrap().style.background,
        Color::Indexed(4)
    );
    assert_eq!(screen.cell(Point { x: 0, y: 1 }).unwrap().text, "c");
    app.tick(
        now + Duration::from_secs(31),
        UNIX_EPOCH + Duration::from_secs(31),
    );
    assert!(!picture(&app).contents().contains("/safe/path"));
    assert_eq!(app.needing(), vec![pane]);
    world.observe(pane, State::Exited(Some(0))).unwrap();
    app.catalog(world.snapshot()).unwrap();
    app.context(pane, Some(context()), now).unwrap();
    assert!(app.needing().is_empty());
    assert!(picture(&app).contents().contains("exited"));
}

#[test]
fn clock() {
    let (mut app, _) = setup();
    let decor = Decor {
        status: client::Status::new(
            &["clock".into(), "prefix".into(), "notice".into()],
            "Asia/Shanghai",
            15,
        )
        .unwrap(),
        ..Default::default()
    };
    app.decorate(decor);
    let now = Instant::now();
    let wall = UNIX_EPOCH + Duration::from_secs(16 * 3600 + 7 * 60);
    app.tick(now, wall);
    app.prefix(Some("C-q"));
    app.status("Configuration unchanged");
    let text = picture(&app).contents();
    assert!(text.contains("00:07"));
    assert!(text.contains("C-q: prefix pending"));
    assert!(text.contains("Configuration unchanged"));
    app.prefix(None);
    app.tick(now + Duration::from_secs(5), wall + Duration::from_secs(60));
    assert!(picture(&app).contents().contains("00:07"));
    app.tick(
        now + Duration::from_secs(15),
        wall + Duration::from_secs(60),
    );
    let text = picture(&app).contents();
    assert!(text.contains("00:08"));
    assert!(!text.contains("prefix pending"));
    assert!(!text.contains("Configuration unchanged"));
}

#[test]
fn clipping() {
    let (mut app, _) = setup();
    let pane = Id::new(3).unwrap();
    let mut state = context();
    state.cwd = format!("/{}\x1b]52;c;unexpected\x07", "中".repeat(1000));
    app.context(pane, Some(state), Instant::now()).unwrap();
    app.status("unsafe\x1b[31mstatus");
    let mut decor = Decor::default();
    decor.header.color = false;
    decor.status.separator = true;
    app.decorate(decor);
    app.resize(Extent::new(2, 1).unwrap());
    let bytes = app.draw().unwrap();
    assert!(!bytes.windows(5).any(|bytes| bytes == b"\x1b]52;"));
    assert!(bytes.len() < 1000);
    assert_eq!(
        picture(&app)
            .cell(Point { x: 0, y: 0 })
            .unwrap()
            .style
            .background,
        Color::Default
    );
    assert!(client::Status::new(&["clock".into()], "local", 15).is_err());
    assert!(client::Status::new(&["clock".into(), "clock".into()], "UTC", 15).is_err());
    assert!(client::Status::new(&[], "UTC", 0).is_err());
    assert!(client::Header::new(&["shell-command".into()]).is_err());
}
#[test]
fn windows() {
    let (mut app, mut world) = setup();
    for _ in 0..31 {
        world
            .apply(app.actor(), Operation::Window(app.target()))
            .unwrap();
    }
    app.catalog(world.snapshot()).unwrap();
    let decor = Decor {
        status: client::Status::new(&["windows".into()], "UTC", 15).unwrap(),
        ..Default::default()
    };
    app.decorate(decor);
    let text = picture(&app).contents();
    assert!(text.contains("bagua w32 control"));
    assert!(text.contains("[32:"));
    app.resize(Extent::new(2, 1).unwrap());
    assert!(app.draw().unwrap().len() < 1000);
}
#[test]
fn heading() {
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    let (mut app, mut world) = setup();
    world
        .apply(
            app.actor(),
            Operation::Split {
                pane: Id::new(3).unwrap(),
                split: workspace::Split {
                    axis: workspace::Axis::Columns,
                    ratio: workspace::Ratio::default(),
                },
            },
        )
        .unwrap();
    app.catalog(world.snapshot()).unwrap();
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    assert_eq!(
        app.mouse(event).unwrap(),
        vec![protocol::Command::Workspace(Operation::Focus(
            Id::new(3).unwrap()
        ))]
    );
    world.disconnect(app.actor()).unwrap();
    app.catalog(world.snapshot()).unwrap();
    assert!(app.mouse(event).unwrap().is_empty());
}

#[test]
fn feedback() {
    let (mut app, mut world) = setup();
    let now = Instant::now();
    let wall = UNIX_EPOCH;
    app.tick(now, wall);
    app.resize(Extent::new(24, 6).unwrap());
    app.error("No window 9; available 1–2");
    assert!(picture(&app).contents().contains("No window 9"));
    app.tick(now + Duration::from_millis(1999), wall);
    assert!(picture(&app).contents().contains("No window 9"));
    app.tick(now + Duration::from_secs(2), wall);
    assert!(!picture(&app).contents().contains("No window 9"));
    app.recovering(true);
    app.tick(now + Duration::from_secs(20), wall);
    assert!(picture(&app).contents().contains("Recovering"));
    app.recovering(false);
    world.disconnect(app.actor()).unwrap();
    app.catalog(world.snapshot()).unwrap();
    app.tick(now + Duration::from_secs(30), wall);
    assert!(picture(&app).contents().contains("Observe"));
}
