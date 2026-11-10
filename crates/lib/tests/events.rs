#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

mod support;
use bagua::Event;
use std::sync::{Arc, Mutex};
use support::{Driver, id};

#[test]
fn connection() {
    let mut driver = Driver::new();
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    driver
        .host
        .events(move |event| sink.lock().unwrap().push(event));
    let actor = driver.host.connect().unwrap();
    driver.host.disconnect(actor).unwrap();
    assert_eq!(
        *events.lock().unwrap(),
        [Event::Connect(actor), Event::Disconnect(actor)]
    );
    assert!(driver.host.connected(driver.actor));
}

#[test]
fn lifecycle() {
    let directory = tempfile::tempdir().unwrap();
    let gate = directory.path().join("finish");
    let mut driver = Driver::new();
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    driver
        .host
        .events(move |event| sink.lock().unwrap().push(event));
    driver.create();
    driver.start(&format!(
        "printf READY; exec 0>&- 1>&- 2>&-; while [ ! -f '{}' ]; do sleep 0.01; done; exit 7",
        gate.display()
    ));
    driver.until(|d| d.contains("READY") && events.lock().unwrap().contains(&Event::Eof(id(3))));
    assert_eq!(
        *events.lock().unwrap(),
        [Event::Start(id(3)), Event::Eof(id(3))]
    );
    assert_eq!(
        driver.host.world().pane(id(3)).unwrap().state(),
        workspace::State::Running
    );
    std::fs::write(gate, b"finish").unwrap();
    driver.until(|d| d.host.world().pane(id(3)).is_none());
    for _ in 0..8 {
        driver.drive();
    }
    assert_eq!(
        *events.lock().unwrap(),
        [
            Event::Start(id(3)),
            Event::Eof(id(3)),
            Event::Exit(
                id(3),
                pty::Exit {
                    code: Some(7),
                    signal: None
                }
            ),
            Event::Release(id(3)),
            Event::Retire(id(3)),
        ]
    );
    assert!(!driver.host.stopped());
}
