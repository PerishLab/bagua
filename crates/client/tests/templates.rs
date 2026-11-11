use client::{App, Decor, Header, Line, Status};
use protocol::{
    Command, Input,
    wire::{Context, Message, Screen},
};
use std::time::{Duration, Instant, UNIX_EPOCH};
use terminal::{Engine, Extent, Point};
use workspace::{Edge, Geometry, Id, Operation, State, World};

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
            Operation::Decorate {
                session: created.session,
                enabled: true,
                edge: Edge::Bottom,
            },
        )
        .unwrap();
    world.observe(created.pane, State::Running).unwrap();
    let mut app = App::new(actor, created.session, Extent::new(100, 8).unwrap());
    app.catalog(world.snapshot()).unwrap();
    let mut engine = Engine::new(Extent::new(100, 7).unwrap(), 0).unwrap();
    engine.feed(b"content\x1b[?1000h").unwrap();
    app.accept(Message::Screen {
        pane: created.pane,
        state: Screen::from(&engine.snapshot()),
    })
    .unwrap();
    let mut status = Status::format(None, "Asia/Shanghai", 15).unwrap();
    status.edge = Edge::Top;
    app.decorate(Decor {
        header: Header::format(None).unwrap(),
        status,
    });
    let now = Instant::now() + Duration::from_secs(3);
    app.tick(now, UNIX_EPOCH + Duration::from_secs(16 * 3600));
    app.context(
        created.pane,
        Some(Context {
            pid: 10,
            session: 10,
            started: 1,
            command: "codex".into(),
            cwd: "/safe/项目".into(),
        }),
        now,
    )
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

#[test]
fn geometry() {
    let (mut app, _) = setup();
    let screen = picture(&app);
    assert!(screen.contents().contains("W1:codex · P1 codex · 项目"));
    assert!(screen.contents().contains("00:00  01-02"));
    assert_eq!(screen.cell(Point { x: 0, y: 2 }).unwrap().text, "c");
    assert_eq!(screen.cursor().position, Point { x: 7, y: 2 });
    let mouse = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: 1,
        row: 2,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };
    let commands = app.mouse(mouse).unwrap();
    assert!(
        matches!(commands.as_slice(),[Command::Send { input:Input::Mouse(value),.. }] if value.position==Point { x:1,y:0 })
    );
    assert!(
        app.mouse(crossterm::event::MouseEvent { row: 9, ..mouse })
            .unwrap()
            .is_empty()
    );
    assert!(
        app.mouse(crossterm::event::MouseEvent { row: 0, ..mouse })
            .unwrap()
            .is_empty()
    );
}

#[test]
fn system() {
    let (mut app, _) = setup();
    app.decorate(Decor {
        header: Header::format(Some("")).unwrap(),
        status: Status::format(Some(vec![Line::new("", "", "").unwrap()]), "UTC", 15).unwrap(),
    });
    app.recovering(true);
    app.prefix(Some("C-q"));
    app.error("important refusal");
    let text = picture(&app).contents();
    assert!(text.contains("Recovering"));
    assert!(text.contains("C-q: prefix pending"));
    assert!(text.contains("important refusal"));
}

#[test]
fn renamed() {
    let (mut app, mut world) = setup();
    world
        .apply(
            app.actor(),
            Operation::Rename {
                target: workspace::Target::Window(Id::new(2).unwrap()),
                name: "manual".into(),
            },
        )
        .unwrap();
    app.catalog(world.snapshot()).unwrap();
    assert!(picture(&app).contents().contains("W1:manual"));
}

#[test]
fn navigation() {
    let (mut app, mut world) = setup();
    world
        .apply(
            app.actor(),
            Operation::Rename {
                target: workspace::Target::Session(app.target()),
                name: "s".repeat(64),
            },
        )
        .unwrap();
    app.catalog(world.snapshot()).unwrap();
    app.resize(Extent::new(20, 8).unwrap());
    let screen = picture(&app);
    let row: String = (0..20)
        .map(|x| screen.cell(Point { x, y: 1 }).unwrap().text)
        .collect();
    assert!(row.contains("1:codex"), "navigation hidden: {row:?}");
}
