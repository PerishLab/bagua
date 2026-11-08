use super::*;

#[test]
fn snapshot() {
    let (mut runner, mut wire, mut world) = setup();
    runner
        .stage(Gesture::Split(workspace::Axis::Columns))
        .unwrap();
    runner
        .stage(Gesture::Input(protocol::Input::Text("next".into())))
        .unwrap();
    runner.drain().unwrap();
    runner.pending().unwrap();
    runner.channel.flush().unwrap();
    assert_eq!(
        wire.take().unwrap().command,
        Command::Describe(Id::new(3).unwrap())
    );
    runner
        .receive(Message::Context {
            request: 1,
            pane: Id::new(3).unwrap(),
            state: Context {
                pid: 20,
                session: 20,
                started: 1,
                command: "sh".into(),
                cwd: "/".into(),
            },
        })
        .unwrap();
    runner
        .receive(Message::Finished {
            request: 1,
            result: Ok(()),
        })
        .unwrap();
    runner.pending().unwrap();
    runner.channel.flush().unwrap();
    let request = wire.take().unwrap();
    let Command::Workspace(operation) = request.command else {
        panic!("expected split")
    };
    let outcome = world.apply(Id::new(100).unwrap(), operation).unwrap();
    runner
        .receive(Message::Reply(protocol::Reply {
            request: 2,
            result: Ok(protocol::Status::Applied(protocol::Outcome::Workspace(
                outcome,
            ))),
        }))
        .unwrap();
    runner
        .receive(Message::Finished {
            request: 2,
            result: Ok(()),
        })
        .unwrap();
    runner.release();
    runner.drain().unwrap();
    assert_eq!(runner.intents.len(), 1);
    runner.pending().unwrap();
    runner.channel.flush().unwrap();
    assert!(
        matches!(wire.take().unwrap().command, Command::Start { pane, .. } if pane == Id::new(4).unwrap())
    );
    runner.receive(Message::Catalog(world.snapshot())).unwrap();
    runner.release();
    assert!(runner.fence.is_some());
    runner
        .receive(Message::Finished {
            request: 3,
            result: Ok(()),
        })
        .unwrap();
    runner.release();
    runner.drain().unwrap();
    runner.pending().unwrap();
    runner.channel.flush().unwrap();
    assert!(
        matches!(wire.take().unwrap().command, Command::Send { pane, input: protocol::Input::Text(text) } if pane == Id::new(4).unwrap() && text == "next")
    );
    assert!(runner.intents.is_empty());
}

#[test]
fn failure() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let (mut runner, mut wire, _) = setup();
    runner
        .stage(Gesture::Split(workspace::Axis::Columns))
        .unwrap();
    runner
        .stage(Gesture::Input(protocol::Input::Text("never-old".into())))
        .unwrap();
    runner.drain().unwrap();
    runner.pending().unwrap();
    runner.channel.flush().unwrap();
    wire.take().unwrap();
    runner
        .receive(Message::Finished {
            request: 1,
            result: Err(Failure::Native.into()),
        })
        .unwrap();
    assert!(runner.paused);
    assert!(runner.intents.is_empty());
    assert_eq!(runner.staged, 0);
    runner
        .stage(Gesture::Input(protocol::Input::Text("blocked".into())))
        .unwrap();
    runner.drain().unwrap();
    runner.pending().unwrap();
    runner.channel.flush().unwrap();
    while let Some(request) = wire.take() {
        assert!(!matches!(request.command, Command::Send { .. }));
    }
    runner
        .key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .unwrap();
    assert!(!runner.paused);
    assert!(runner.intents.is_empty());
}

#[test]
fn bounds() {
    let (mut runner, _, _) = setup();
    for _ in 0..16 {
        runner
            .stage(Gesture::Input(protocol::Input::Text("x".repeat(4096))))
            .unwrap();
    }
    assert_eq!(runner.staged, 65_536);
    assert!(
        runner
            .stage(Gesture::Input(protocol::Input::Text("overflow".into())))
            .is_err()
    );
    assert_eq!(runner.staged, 65_536);
    assert_eq!(runner.intents.len(), 16);
}

#[test]
fn transfer() {
    let (mut runner, mut wire, mut world) = setup();
    world
        .apply(
            Id::new(200).unwrap(),
            Operation::Join {
                session: Id::new(1).unwrap(),
                viewport: workspace::Geometry::new(80, 24).unwrap(),
            },
        )
        .unwrap();
    runner.receive(Message::Catalog(world.snapshot())).unwrap();
    runner.stage(Gesture::Transfer).unwrap();
    runner
        .stage(Gesture::Input(protocol::Input::Text(
            "never-write-after-transfer".into(),
        )))
        .unwrap();
    runner.drain().unwrap();
    runner.pending().unwrap();
    runner.channel.flush().unwrap();
    let request = wire.take().unwrap();
    let Command::Workspace(operation) = request.command else {
        panic!("expected transfer")
    };
    let outcome = world.apply(Id::new(100).unwrap(), operation).unwrap();
    runner
        .receive(Message::Reply(protocol::Reply {
            request: 1,
            result: Ok(protocol::Status::Applied(protocol::Outcome::Workspace(
                outcome,
            ))),
        }))
        .unwrap();
    runner.receive(Message::Catalog(world.snapshot())).unwrap();
    runner
        .receive(Message::Finished {
            request: 1,
            result: Ok(()),
        })
        .unwrap();
    runner.release();
    runner.drain().unwrap();
    assert!(!runner.app.controller());
    assert!(runner.intents.is_empty());
    assert!(
        runner
            .queue
            .iter()
            .all(|item| !matches!(item.command, Command::Send { .. }))
    );
}

#[test]
fn unknown() {
    let (mut runner, mut wire, _) = setup();
    runner.stage(Gesture::Index(1)).unwrap();
    runner
        .stage(Gesture::Input(protocol::Input::Text(
            "unknown-must-not-write".into(),
        )))
        .unwrap();
    runner.drain().unwrap();
    runner.pending().unwrap();
    runner.channel.flush().unwrap();
    let request = wire.take().unwrap();
    runner
        .receive(Message::Finished {
            request: request.id,
            result: Ok(()),
        })
        .unwrap();
    runner.release();
    runner.drain().unwrap();
    assert!(runner.paused);
    assert!(runner.fence.is_some());
    assert!(runner.intents.is_empty());
    assert!(wire.take().is_none());
}
