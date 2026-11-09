#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

mod support;
use protocol::{Command, Input, wire::Message};
use support::{Driver, id};

#[test]
fn automatic() {
    let mut driver = Driver::new();
    driver.create();
    driver.start("stty -echo; printf READY; IFS= read -r line; printf TAIL; exit 7");
    driver.until(|d| d.contains("READY"));
    driver
        .call(Command::Send {
            pane: id(3),
            input: Input::Paste("finish\n".into()),
        })
        .unwrap();
    driver.until(|d| d.host.world().pane(id(3)).is_none());
    assert!(driver.host.world().session(id(1)).is_none());
    assert!(driver.messages.iter().any(|m| matches!(m, Message::Process { pane, exited: true, code: Some(7), .. } if *pane == id(3))));
}

#[test]
fn background() {
    let directory = tempfile::tempdir().unwrap();
    let gate = directory.path().join("finish");
    let mut driver = Driver::new();
    driver.create();
    driver.start(&format!(
        "printf READY; while [ ! -f '{}' ]; do sleep 0.01; done; exit 0",
        gate.display()
    ));
    driver.until(|d| d.contains("READY"));
    driver.host.disconnect(driver.actor).unwrap();
    assert_eq!(
        driver.host.world().pane(id(3)).unwrap().state(),
        workspace::State::Running
    );
    std::fs::write(gate, b"finish").unwrap();
    driver.until(|d| d.host.world().pane(id(3)).is_none());
    assert!(driver.host.world().session(id(1)).is_none());
    assert!(!driver.host.stopped());
}

#[test]
fn eof() {
    let directory = tempfile::tempdir().unwrap();
    let gate = directory.path().join("finish");
    let mut driver = Driver::new();
    driver.create();
    driver.start(&format!(
        "exec 0>&- 1>&- 2>&-; while [ ! -f '{}' ]; do sleep 0.01; done; exit 0",
        gate.display()
    ));
    driver.until(|d| {
        d.messages.iter().any(|m| {
            matches!(
                m,
                Message::Process {
                    eof: true,
                    exited: false,
                    ..
                }
            )
        })
    });
    assert_eq!(
        driver.host.world().pane(id(3)).unwrap().state(),
        workspace::State::Running
    );
    std::fs::write(gate, b"finish").unwrap();
    driver.until(|d| d.host.world().session(id(1)).is_none());
}

#[test]
fn signal() {
    let mut driver = Driver::new();
    driver.create();
    driver.start("stty -echo; printf READY; IFS= read -r line; kill -TERM $$");
    driver.until(|d| d.contains("READY"));
    driver
        .call(Command::Send {
            pane: id(3),
            input: Input::Paste("finish\n".into()),
        })
        .unwrap();
    driver.until(|d| d.host.world().pane(id(3)).is_none());
    assert!(driver.messages.iter().any(|m| matches!(
        m,
        Message::Process {
            exited: true,
            signal: Some(15),
            ..
        }
    )));
}

#[test]
fn survivor() {
    use support::program;
    use workspace::{Axis, Operation, Ratio, Split, Target};
    let mut driver = Driver::new();
    driver.create();
    driver.start("stty -echo; printf SURVIVOR; while IFS= read -r line; do stty size; done");
    driver.until(|d| d.contains("SURVIVOR"));
    driver
        .call(Command::Workspace(Operation::Split {
            pane: id(3),
            split: Split {
                axis: Axis::Columns,
                ratio: Ratio::default(),
            },
        }))
        .unwrap();
    driver
        .call(Command::Subscribe {
            target: Target::Pane(id(4)),
            after: None,
        })
        .unwrap();
    driver
        .call(Command::Start {
            pane: id(4),
            program: program("stty -echo; printf SPLITREADY; IFS= read -r line; exit 0"),
        })
        .unwrap();
    driver.until(|d| d.contains("SPLITREADY"));
    driver
        .call(Command::Send {
            pane: id(4),
            input: Input::Paste("finish\n".into()),
        })
        .unwrap();
    driver.until(|d| d.host.world().pane(id(4)).is_none());
    assert_eq!(
        driver.host.world().pane(id(3)).unwrap().state(),
        workspace::State::Running
    );
    assert_eq!(
        driver
            .host
            .world()
            .session(id(1))
            .unwrap()
            .windows()
            .next()
            .unwrap()
            .focus(),
        id(3)
    );
    driver
        .call(Command::Send {
            pane: id(3),
            input: Input::Paste("size\n".into()),
        })
        .unwrap();
    driver.until(|d| d.contains("24 80"));
}
