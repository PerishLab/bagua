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
    assert!(runner.recovery.is_none());
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
    assert!(runner.recovery.is_none());
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

#[test]
fn duplicate() {
    let (mut runner, _, _) = setup();
    let fault = protocol::wire::Fault {
        code: "io".into(),
        message: "executable missing".into(),
    };
    runner.jobs.insert(1, Job::Start(Id::new(3).unwrap()));
    runner
        .receive(Message::Finished {
            request: 1,
            result: Err(fault.clone()),
        })
        .unwrap();
    runner
        .receive(Message::Process {
            pane: Id::new(3).unwrap(),
            io: false,
            closing: false,
            exited: false,
            code: None,
            signal: None,
            eof: false,
            fault: Some(fault),
        })
        .unwrap();
    assert!(
        runner
            .app
            .draw()
            .unwrap()
            .windows(10)
            .any(|bytes| bytes == b"Start pane")
    );
}

#[test]
fn refusal() {
    for retired in [false, true] {
        let (mut runner, _, mut world) = setup();
        world
            .observe(Id::new(3).unwrap(), workspace::State::Running)
            .unwrap();
        if retired {
            world
                .apply(
                    Id::new(100).unwrap(),
                    Operation::Close(workspace::Target::Session(Id::new(1).unwrap())),
                )
                .unwrap();
        }
        runner.app.catalog(world.snapshot()).unwrap();
        runner.app.status("Configuration unchanged");
        runner.jobs.insert(
            1,
            Job::Metadata {
                pane: Id::new(3).unwrap(),
                at: Instant::now(),
                received: false,
            },
        );
        assert!(!runner.settled());
        runner
            .receive(Message::Reply(protocol::Reply {
                request: 1,
                result: Err(Failure::Workspace(workspace::Failure::Missing)),
            }))
            .unwrap();
        assert!(runner.jobs.is_empty());
        assert!(runner.settled());
        assert!(runner.recovery.is_none());
        assert_eq!(runner.app.done(), retired);
        if !retired {
            assert!(
                runner
                    .app
                    .draw()
                    .unwrap()
                    .windows(23)
                    .any(|bytes| bytes == b"Configuration unchanged")
            );
        }
    }
}

#[test]
fn admission() {
    let (mut runner, _, _) = setup();
    runner.jobs.insert(
        1,
        Job::Metadata {
            pane: Id::new(3).unwrap(),
            at: Instant::now(),
            received: false,
        },
    );
    runner
        .receive(Message::Reply(protocol::Reply {
            request: 1,
            result: Ok(protocol::Status::Admitted),
        }))
        .unwrap();
    assert!(!runner.settled());
    assert_eq!(runner.jobs.len(), 1);
    runner
        .receive(Message::Finished {
            request: 1,
            result: Err(Failure::Unavailable.into()),
        })
        .unwrap();
    assert!(runner.jobs.is_empty());
    assert!(runner.settled());
}
