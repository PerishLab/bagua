mod decor;
mod gating;
mod recovery;
use super::*;
use bagua::ipc::Connection;
use protocol::wire::{Context, Decoder, REQUEST};
use std::{
    io::Read,
    os::unix::{
        fs::PermissionsExt,
        net::{UnixListener, UnixStream},
    },
};

struct Wire {
    stream: UnixStream,
    decoder: Decoder,
    _directory: tempfile::TempDir,
}

impl Wire {
    fn take(&mut self) -> Option<Request> {
        let mut bytes = [0; 4096];
        match self.stream.read(&mut bytes) {
            Ok(count) => self.decoder.push(&bytes[..count]).unwrap(),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => panic!("wire read failed: {error}"),
        }
        self.decoder.take().unwrap()
    }
}

fn setup() -> (Runner, Wire, workspace::World) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = directory.path().join("wire.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let local = Connection::connect(&path).unwrap();
    let (stream, _) = listener.accept().unwrap();
    stream.set_nonblocking(true).unwrap();
    let actor = Id::new(100).unwrap();
    let mut world = workspace::World::default();
    let geometry = workspace::Geometry::new(80, 24).unwrap();
    world
        .apply(
            actor,
            Operation::Enter(workspace::Entry {
                name: "workspace".into(),
                viewport: geometry,
                create: true,
                observe: false,
                fixed: false,
            }),
        )
        .unwrap();
    let mut app = App::new(
        actor,
        Id::new(1).unwrap(),
        terminal::Extent::new(80, 24).unwrap(),
    );
    app.catalog(world.snapshot()).unwrap();
    let program = Program {
        executable: "/bin/sh".into(),
        args: Vec::new(),
        cwd: "/".into(),
        environment: BTreeMap::new(),
    };
    (
        Runner::new(local.channel().unwrap(), app, 1, Some(program)),
        Wire {
            stream,
            decoder: Decoder::new(REQUEST),
            _directory: directory,
        },
        world,
    )
}

fn split(axis: workspace::Axis) -> Command {
    Command::Workspace(Operation::Split {
        pane: Id::new(3).unwrap(),
        split: workspace::Split {
            axis,
            ratio: workspace::Ratio::default(),
        },
    })
}

#[test]
fn order() {
    let (mut runner, mut wire, _) = setup();
    let first = split(workspace::Axis::Columns);
    let second = split(workspace::Axis::Rows);
    runner.enqueue(first.clone());
    runner.enqueue(second);
    runner.pending().unwrap();
    runner.channel.flush().unwrap();
    assert_eq!(
        wire.take().unwrap().command,
        Command::Describe(Id::new(3).unwrap())
    );
    runner.pending().unwrap();
    runner.channel.flush().unwrap();
    assert!(wire.take().is_none());
    runner
        .receive(Message::Context {
            request: 1,
            pane: Id::new(3).unwrap(),
            state: Context {
                pid: 20,
                session: 20,
                started: 1,
                command: "sh".into(),
                cwd: "/tmp".into(),
            },
        })
        .unwrap();
    runner.pending().unwrap();
    runner.channel.flush().unwrap();
    let request = wire.take().unwrap();
    assert_eq!(request.id, 2);
    assert_eq!(request.command, first);
    assert!(matches!(runner.jobs.get(&2), Some(Job::Spawn(program)) if program.cwd == "/tmp"));
    runner.pending().unwrap();
    runner.channel.flush().unwrap();
    assert_eq!(
        wire.take().unwrap().command,
        Command::Describe(Id::new(3).unwrap())
    );
}

#[test]
fn refusal() {
    let (mut runner, mut wire, _) = setup();
    runner.enqueue(split(workspace::Axis::Columns));
    runner.enqueue(split(workspace::Axis::Rows));
    runner.pending().unwrap();
    runner.channel.flush().unwrap();
    wire.take().unwrap();
    runner
        .receive(Message::Finished {
            request: 1,
            result: Err(Failure::Native.into()),
        })
        .unwrap();
    assert_eq!(runner.queue.len(), 1);
    runner.pending().unwrap();
    runner.channel.flush().unwrap();
    assert_eq!(
        wire.take().unwrap().command,
        Command::Describe(Id::new(3).unwrap())
    );
    assert!(wire.take().is_none());
}

#[test]
fn prefix() {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    let (mut runner, _, _) = setup();
    let control = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL);
    runner.key(control).unwrap();
    assert!(runner.leader);
    let release = KeyEvent::new_with_kind(
        KeyCode::Char('g'),
        KeyModifiers::CONTROL,
        KeyEventKind::Release,
    );
    runner.key(release).unwrap();
    assert!(runner.leader);
    runner.event(Event::Paste("=\\".into())).unwrap();
    assert!(!runner.leader);
    runner.drain().unwrap();
    assert_eq!(runner.queue.len(), 1);
    assert!(
        runner
            .queue
            .iter()
            .all(|item| matches!(item.command, Command::Send { .. }))
    );
}
