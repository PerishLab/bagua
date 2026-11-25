#![cfg(all(target_os = "windows", target_arch = "x86_64"))]

#[path = "support/win.rs"]
mod support;

use bagua::Event;
use protocol::{Command, Failure, Input, wire::Message};
use std::sync::{Arc, Mutex};
use support::{Driver, id};
use workspace::{Operation, State, Target};

#[test]
fn lifecycle() {
    let mut driver = Driver::new();
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);
    driver
        .host
        .events(move |event| sink.lock().unwrap().push(event));
    driver.create();
    driver.start(
        "[Console]::Write('READY'); $line = [Console]::ReadLine(); [Console]::Write('GOT:' + $line); exit 7",
    );
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
            after: None,
        })
        .unwrap();
    driver
        .call(Command::Send {
            pane: id(3),
            input: Input::Paste("hello\r\n".into()),
        })
        .unwrap();
    driver.until(|d| d.host.world().pane(id(3)).is_none());
    assert!(driver.host.world().session(id(1)).is_none());
    assert!(driver.messages.iter().any(|m| matches!(
        m,
        Message::Process {
            exited: true,
            code: Some(7),
            signal: None,
            ..
        }
    )));
    let recorded = events.lock().unwrap();
    assert!(recorded.contains(&Event::Start(id(3))));
    assert!(recorded.contains(&Event::Retire(id(3))));
}

#[test]
fn admission() {
    let mut driver = Driver::new();
    driver.create();
    driver.start("[Console]::Write('READY'); Start-Sleep -Seconds 30");
    driver.until(|d| d.contains("READY"));
    for _ in 1..32 {
        driver.call(Command::create(80, 24).unwrap()).unwrap();
    }
    let close = driver.submit(Command::Workspace(Operation::Close(Target::Session(id(1)))));
    assert_eq!(
        driver.call(Command::create(80, 24).unwrap()),
        Err(Failure::Capacity)
    );
    driver.until(|d| {
        d.messages.iter().any(
            |m| matches!(m, Message::Finished { request, result: Ok(()) } if *request == close),
        )
    });
    driver.call(Command::create(80, 24).unwrap()).unwrap();
    assert!(driver.host.world().session(id(97)).is_some());
}

#[test]
fn backpressure() {
    let mut driver = Driver::new();
    driver.create();
    driver.start(
        "[Console]::Write('READY'); $line = [Console]::ReadLine(); [Console]::Write('LEN:' + $line.Length); exit 0",
    );
    driver.until(|d| d.contains("READY"));
    for _ in 0..16 {
        driver.submit(Command::Send {
            pane: id(3),
            input: Input::Text("a".repeat(4096)),
        });
    }
    driver.submit(Command::Send {
        pane: id(3),
        input: Input::Text("extra".into()),
    });
    driver.until(|driver| {
        driver
            .messages
            .iter()
            .any(|m| matches!(m, Message::Reply(reply) if reply.result == Err(Failure::Capacity)))
    });
    driver.host.disconnect(driver.actor).unwrap();
    driver.actor = driver.host.connect().unwrap();
    driver.join();
    driver
        .call(Command::Workspace(Operation::Acquire {
            session: id(1),
            fixed: None,
        }))
        .unwrap();
    driver
        .call(Command::Subscribe {
            target: Target::Pane(id(3)),
            after: None,
        })
        .unwrap();
    driver.until(|d| !d.host.pending(id(1)));
    driver
        .call(Command::Send {
            pane: id(3),
            input: Input::Key(terminal::Key::Enter),
        })
        .unwrap();
    driver.until(|d| d.contains("LEN:65536"));
}
