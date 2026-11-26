use super::{Stop, peer};
use bagua::ipc::{Connection, Service};
use protocol::{
    Command, Request,
    wire::{Message, Program},
};
use std::{
    collections::BTreeMap,
    fs,
    process::Command as Exec,
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};
use workspace::{Geometry, Id, Operation};

#[test]
fn inspect() {
    let dir = peer::directory();
    let nested = dir.path().join("child");
    fs::create_dir(&nested).unwrap();
    let path = dir.path().join("host.sock");
    let mut service = Service::bind(&path).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let session = Id::new(1).unwrap();
    let pane = Id::new(3).unwrap();
    std::thread::scope(|scope| {
        let task = scope.spawn(|| service.run(&stop).unwrap());
        let guard = Stop(Arc::clone(&stop));
        let mut connection = Connection::connect(&path).unwrap();
        let _ = connection.receive().unwrap();
        setup(&mut connection, (session, pane), dir.path());
        let text = wait(&path);
        assert!(
            text.starts_with("pane 3 | running | 80x24 | cursor "),
            "{text:?}"
        );
        let context = describe(&mut connection, pane);
        assert!(context.pid > 0 && context.session > 0 && context.started > 0);
        assert!(context.command.eq_ignore_ascii_case("cmd"));
        assert!(context.cwd.to_ascii_lowercase().ends_with("child"));
        drop(guard);
        task.join().unwrap();
    });
}

fn setup(connection: &mut Connection, ids: (Id, Id), root: &std::path::Path) {
    let (session, pane) = ids;
    let commands = [
        Command::create(80, 24).unwrap(),
        Command::Workspace(Operation::Join {
            session,
            viewport: Geometry::new(80, 24).unwrap(),
        }),
        Command::Workspace(Operation::Acquire {
            session,
            fixed: None,
        }),
        Command::Start {
            pane,
            program: Program {
                executable: peer::comspec(),
                args: vec![
                    "/Q".into(),
                    "/K".into(),
                    "cd /D child & echo BAGUA_TEXT_OK".into(),
                ],
                cwd: root.to_string_lossy().into_owned(),
                environment: BTreeMap::from([(
                    "SYSTEMROOT".into(),
                    peer::systemroot().to_string_lossy().into_owned(),
                )]),
            },
        },
    ];
    for (index, command) in commands.into_iter().enumerate() {
        connection
            .send(&Request::identified(index as u64 + 1, command).unwrap())
            .unwrap();
        loop {
            if let Message::Finished { request, result } = connection.receive().unwrap()
                && request == index as u64 + 1
            {
                assert!(result.is_ok(), "{result:?}");
                break;
            }
        }
    }
}

fn wait(path: &std::path::Path) -> String {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let output = Exec::new(env!("CARGO_BIN_EXE_bagua"))
            .args(["--no-config", "--socket"])
            .arg(path)
            .args(["--session", "1", "inspect", "3", "--text"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let body = String::from_utf8(output.stdout).unwrap();
        if body.contains("BAGUA_TEXT_OK") {
            return body;
        }
        assert!(
            Instant::now() < deadline,
            "inspect text missed output: {body:?}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn describe(connection: &mut Connection, pane: Id) -> protocol::wire::Context {
    connection
        .send(&Request::identified(10, Command::Describe(pane)).unwrap())
        .unwrap();
    let mut observed = None;
    loop {
        match connection.receive().unwrap() {
            Message::Context { state, .. } => observed = Some(state),
            Message::Finished {
                request: 10,
                result,
            } => {
                assert!(result.is_ok(), "{result:?}");
                break;
            }
            _ => {}
        }
    }
    observed.expect("missing context message")
}
