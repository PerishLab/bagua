#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

mod support;
use protocol::{Command, Failure, Input, Request, wire::Message};
use support::{Driver, id, program};
use workspace::{Geometry, Operation, State, Target};

#[test]
fn disconnect() {
    let mut driver = Driver::new();
    driver.create();
    driver
        .start("stty -echo; printf READY; IFS= read -r line; printf 'RESULT:%s' \"$line\"; exit 7");
    driver.until(|d| d.contains("READY"));
    driver.host.disconnect(driver.actor).unwrap();
    assert_eq!(
        driver.host.world().session(id(1)).unwrap().controller(),
        None
    );
    assert_eq!(
        driver.host.world().pane(id(3)).unwrap().state(),
        State::Running
    );
    driver.actor = driver.host.connect().unwrap();
    driver.join();
    assert_eq!(
        driver.call(Command::Send {
            pane: id(3),
            input: Input::Text("hello".into())
        }),
        Err(Failure::Workspace(workspace::Failure::Observer))
    );
    driver
        .call(Command::Workspace(Operation::Acquire {
            session: id(1),
            fixed: None,
        }))
        .unwrap();
    driver
        .call(Command::Subscribe {
            target: Target::Pane(id(3)),
            after: Some(terminal::Revision::new(9999)),
        })
        .unwrap();
    driver
        .call(Command::Send {
            pane: id(3),
            input: Input::Text("hello".into()),
        })
        .unwrap();
    driver
        .call(Command::Send {
            pane: id(3),
            input: Input::Key(terminal::Key::Enter),
        })
        .unwrap();
    driver.until(|d| d.contains("RESULT:hello"));
    driver.until(|d| d.host.world().pane(id(3)).unwrap().state() == State::Exited(Some(7)));
    driver.call(Command::Inspect(id(3))).unwrap();
}

#[test]
fn slow() {
    let mut driver = Driver::new();
    driver.create();
    let observer = driver.host.connect().unwrap();
    driver
        .host
        .submit(observer, Request::new(Command::attach(1, 80, 24).unwrap()))
        .unwrap();
    driver
        .host
        .submit(
            observer,
            Request::identified(
                2,
                Command::Subscribe {
                    target: Target::Pane(id(3)),
                    after: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
    driver
        .call(Command::Subscribe {
            target: Target::Pane(id(3)),
            after: None,
        })
        .unwrap();
    driver
        .call(Command::Start {
            pane: id(3),
            program: program("stty raw -echo; head -c 131072 /dev/zero; printf END"),
        })
        .unwrap();
    driver.until(|d| d.host.world().pane(id(3)).unwrap().state() == State::Exited(Some(0)));
    driver.call(Command::Inspect(id(3))).unwrap();
    driver.until(|d| d.contains("END"));
    assert!(
        driver
            .messages
            .iter()
            .any(|m| matches!(m, Message::Screen { state, .. } if state.bytes == 131075))
    );
    assert!(driver.host.connected(observer));
}

#[test]
fn presentation() {
    let mut driver = Driver::new();
    driver.create();
    driver.start("stty raw -echo; head -c 131072 /dev/zero; printf END");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !driver.contains("END") {
        assert!(std::time::Instant::now() < deadline);
        driver
            .call(Command::Workspace(Operation::Focus(id(3))))
            .unwrap();
    }
}

#[test]
fn query() {
    let mut driver = Driver::new();
    driver.create();
    driver.start(r"stty raw -echo; printf '\033[5n'; od -An -tx1 -N4");
    driver.until(|d| d.contains("1b 5b 30 6e"));
    driver.until(|d| d.host.world().pane(id(3)).unwrap().state() == State::Exited(Some(0)));
}

#[test]
fn retirement() {
    let mut driver = Driver::new();
    driver.create();
    driver.start("trap '' HUP; printf READY; exec /bin/sleep 60");
    driver.until(|d| d.contains("READY"));
    let request = driver.submit(Command::Workspace(Operation::Close(Target::Pane(id(3)))));
    assert!(driver.host.world().pane(id(3)).is_none());
    assert!(driver.host.pending(driver.actor));
    driver.call(Command::Workspace(Operation::Read)).unwrap();
    driver.until(|d| {
        d.messages
            .iter()
            .any(|m| matches!(m, Message::Finished { request: r, result: Ok(()) } if *r == request))
    });
    assert!(!driver.host.pending(driver.actor));
    driver.call(Command::Shutdown).unwrap();
    assert!(driver.host.stopped());
}

#[test]
fn failure() {
    let mut driver = Driver::new();
    driver.create();
    let mut launch = program("exit 0");
    let directory = tempfile::tempdir().unwrap();
    launch.executable = directory.path().join("missing").to_str().unwrap().into();
    assert_eq!(
        driver.call(Command::Start {
            pane: id(3),
            program: launch
        }),
        Err(Failure::Native)
    );
    assert_eq!(
        driver.host.world().pane(id(3)).unwrap().state(),
        State::Failed
    );
    driver
        .call(Command::Workspace(Operation::Close(Target::Pane(id(3)))))
        .unwrap();
}

#[test]
fn resize() {
    let mut driver = Driver::new();
    driver.create();
    driver.start(
        "trap 'stty size; printf WINCH' WINCH; printf READY; while :; do read -r line; done",
    );
    driver.until(|d| d.contains("READY"));
    driver
        .call(Command::Workspace(Operation::Size {
            session: id(1),
            sizing: workspace::Sizing::Fixed(Geometry::new(100, 30).unwrap()),
        }))
        .unwrap();
    driver.until(|d| d.contains("30 100"));
    driver
        .call(Command::Workspace(Operation::Split {
            pane: id(3),
            split: workspace::Split {
                axis: workspace::Axis::Columns,
                ratio: workspace::Ratio::default(),
            },
        }))
        .unwrap();
    driver.until(|d| d.contains("30 49"));
    driver
        .call(Command::Workspace(Operation::Close(Target::Session(id(1)))))
        .unwrap();
}
