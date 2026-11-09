use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn navigation() {
    let (mut runner, mut wire, _) = setup();
    for gesture in [
        Gesture::Index(9),
        Gesture::Index(1),
        Gesture::Pane,
        Gesture::Next,
    ] {
        runner.stage(gesture).unwrap();
        runner
            .stage(Gesture::Input(protocol::Input::Text("ok".into())))
            .unwrap();
        runner.drain().unwrap();
        assert!(!runner.fenced());
        assert!(runner.recovery.is_none());
    }
    runner.pending().unwrap();
    runner.channel.flush().unwrap();
    assert!(matches!(wire.take().unwrap().command,
        Command::Send { pane, input: protocol::Input::Text(text) }
        if pane.value() == 3 && text == "okokokok"));
    assert!(wire.take().is_none());
}

#[test]
fn synchronize() {
    let (mut runner, mut wire, world) = setup();
    runner.stage(Gesture::Acquire).unwrap();
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
    runner
        .key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .unwrap();
    runner
        .stage(Gesture::Input(protocol::Input::Text("never".into())))
        .unwrap();
    assert!(runner.recovery.is_some());
    assert!(runner.intents.is_empty());
    runner.recover().unwrap();
    runner.channel.flush().unwrap();
    let request = wire.take().unwrap();
    assert_eq!(request.command, Command::Workspace(Operation::Read));
    runner
        .receive(Message::Reply(protocol::Reply {
            request: request.id,
            result: Ok(protocol::Status::Applied(protocol::Outcome::Workspace(
                workspace::Outcome::Snapshot(world.snapshot()),
            ))),
        }))
        .unwrap();
    assert!(runner.recovery.is_some());
    runner
        .receive(Message::Finished {
            request: request.id,
            result: Ok(()),
        })
        .unwrap();
    assert!(runner.recovery.is_none());
    assert!(runner.fence.is_none());
    assert!(runner.intents.is_empty());
    runner
        .stage(Gesture::Input(protocol::Input::Text("fresh".into())))
        .unwrap();
    runner.drain().unwrap();
    runner.pending().unwrap();
    runner.channel.flush().unwrap();
    assert!(matches!(wire.take().unwrap().command,
        Command::Send { input: protocol::Input::Text(text), .. } if text == "fresh"));
}

#[test]
fn bounded() {
    let (mut runner, _, _) = setup();
    runner.resolve(super::super::recovery::Resolution::Unknown(
        "missing".into(),
    ));
    runner.recovery.as_mut().unwrap().expire();
    assert!(runner.recover().is_err());
}

#[test]
fn escape() {
    let (mut runner, _, _) = setup();
    let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    runner.key(escape).unwrap();
    runner.drain().unwrap();
    assert!(matches!(
        runner.queue.pop_front().unwrap().command,
        Command::Send {
            input: protocol::Input::Key(terminal::Key::Escape),
            ..
        }
    ));
    runner
        .key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL))
        .unwrap();
    runner.key(escape).unwrap();
    assert!(!runner.leader);
    assert!(runner.intents.is_empty());
}

#[test]
fn refusal() {
    let (mut runner, mut wire, _) = setup();
    runner.stage(Gesture::Acquire).unwrap();
    runner
        .stage(Gesture::Input(protocol::Input::Text("never".into())))
        .unwrap();
    runner.stage(Gesture::Window).unwrap();
    runner
        .stage(Gesture::Input(protocol::Input::Text("never-child".into())))
        .unwrap();
    runner.drain().unwrap();
    runner.pending().unwrap();
    runner.channel.flush().unwrap();
    let request = wire.take().unwrap();
    runner
        .receive(Message::Reply(protocol::Reply {
            request: request.id,
            result: Err(Failure::Workspace(workspace::Failure::Busy)),
        }))
        .unwrap();
    assert!(runner.recovery.is_none());
    assert!(runner.intents.is_empty());
    runner
        .stage(Gesture::Input(protocol::Input::Text("fresh".into())))
        .unwrap();
    runner
        .receive(Message::Finished {
            request: request.id,
            result: Err(Failure::Workspace(workspace::Failure::Busy).into()),
        })
        .unwrap();
    runner.drain().unwrap();
    runner.pending().unwrap();
    runner.channel.flush().unwrap();
    assert!(matches!(wire.take().unwrap().command,
        Command::Send { pane, input: protocol::Input::Text(text) }
        if pane.value() == 3 && text == "fresh"));
    assert!(wire.take().is_none());
}

#[test]
fn fragmented() {
    let (mut runner, mut wire, _) = setup();
    runner
        .stage(Gesture::Split(workspace::Axis::Columns))
        .unwrap();
    runner.drain().unwrap();
    runner.pending().unwrap();
    runner.channel.flush().unwrap();
    let request = wire.take().unwrap();
    runner.keyboard.push(b"\x1b[200~never").unwrap();
    assert!(runner.keyboard.next(Instant::now()).is_none());
    runner
        .receive(Message::Finished {
            request: request.id,
            result: Err(Failure::Native.into()),
        })
        .unwrap();
    for byte in b"-old\x1b[201~z" {
        runner.keyboard.push(&[*byte]).unwrap();
        while let Some(event) = runner.keyboard.next(Instant::now()) {
            if let Ok(event) = event {
                runner.event(event).unwrap();
            }
        }
    }
    runner.drain().unwrap();
    runner.pending().unwrap();
    runner.channel.flush().unwrap();
    assert!(matches!(wire.take().unwrap().command,
        Command::Send { pane, input: protocol::Input::Text(text) }
        if pane.value() == 3 && text == "z"));
    assert!(wire.take().is_none());
}
