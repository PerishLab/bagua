use super::*;

#[test]
fn background() {
    let (mut runner, _, _) = setup();
    runner.app.status("Configuration unchanged");
    runner.jobs.insert(
        1,
        Job::Metadata {
            pane: Id::new(3).unwrap(),
            at: Instant::now(),
            received: false,
        },
    );
    runner
        .receive(Message::Finished {
            request: 1,
            result: Err(Failure::Unavailable.into()),
        })
        .unwrap();
    assert!(!runner.paused);
    assert!(runner.jobs.is_empty());
    assert!(
        runner
            .app
            .draw()
            .unwrap()
            .windows(23)
            .any(|bytes| bytes == b"Configuration unchanged")
    );
}

#[test]
fn rejection() {
    let (mut runner, _, _) = setup();
    let command = Command::Workspace(Operation::Header {
        session: runner.app.target(),
        enabled: true,
    });
    assert!(
        runner
            .submit(command, Job::Header { refused: false })
            .unwrap()
    );
    runner
        .stage(Gesture::Input(protocol::Input::Text("x".into())))
        .unwrap();
    runner
        .receive(Message::Reply(protocol::Reply {
            request: 1,
            result: Err(Failure::Workspace(workspace::Failure::Geometry)),
        }))
        .unwrap();
    runner
        .receive(Message::Finished {
            request: 1,
            result: Err(Failure::Workspace(workspace::Failure::Geometry).into()),
        })
        .unwrap();
    assert!(!runner.paused);
    assert_eq!(runner.intents.len(), 1);
    assert!(
        runner
            .app
            .draw()
            .unwrap()
            .windows(26)
            .any(|bytes| bytes == b"Header reservation refused")
    );
    runner.drain().unwrap();
    assert!(
        matches!(&runner.queue[0].command,Command::Send {pane,input:protocol::Input::Text(text)} if pane.value()==3 && text=="x")
    );
}

#[test]
fn context() {
    let (mut runner, _, _) = setup();
    runner.jobs.insert(
        1,
        Job::Metadata {
            pane: Id::new(3).unwrap(),
            at: Instant::now(),
            received: false,
        },
    );
    assert!(
        runner
            .receive(Message::Finished {
                request: 1,
                result: Ok(())
            })
            .is_err()
    );
}
