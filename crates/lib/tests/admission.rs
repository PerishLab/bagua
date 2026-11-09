#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

mod support;
use protocol::{Command, Failure, Input, Request, wire::Message};
use support::{Driver, id};
use workspace::{Operation, Target};

#[test]
fn atomic() {
    let mut driver = Driver::new();
    assert_eq!(
        driver.call(Command::create(257, 64).unwrap()),
        Err(Failure::Capacity)
    );
    assert_eq!(driver.host.world().revision(), 0);
    driver.create();
    let revision = driver.host.world().revision();
    driver
        .host
        .submit(
            driver.actor,
            Request::identified(1, Command::create(80, 24).unwrap()).unwrap(),
        )
        .unwrap();
    driver.drive();
    assert_eq!(driver.host.world().revision(), revision);
    driver
        .host
        .submit(
            driver.actor,
            Request {
                version: 0,
                id: 99,
                command: Command::create(80, 24).unwrap(),
            },
        )
        .unwrap();
    driver.drive();
    assert_eq!(driver.host.world().revision(), revision);
    assert_eq!(driver.host.world().snapshot().sessions().count(), 1);
}

#[test]
fn custody() {
    let mut driver = Driver::new();
    driver.create();
    driver.start("trap '' HUP; printf READY; exec /bin/sleep 60");
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
fn input() {
    let mut driver = Driver::new();
    driver.create();
    driver.start("stty raw -echo; printf READY; head -c 65536 | wc -c; head -c 1 >/dev/null");
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
        .call(Command::Subscribe {
            target: Target::Pane(id(3)),
            after: None,
        })
        .unwrap();
    driver.until(|d| d.contains("65536"));
}
