use super::*;
use crate::host::pane::Pane;
use terminal::Extent;

fn host() -> Host {
    let mut host = Host::configured([1; 16], crate::config::History::default()).unwrap();
    let mut pane = Pane::new(Extent::new(8, 3).unwrap(), host.history).unwrap();
    pane.engine
        .feed(b"zero\r\none\r\ntwo\r\nthree\r\nfour\r\nfive")
        .unwrap();
    host.panes.insert(Id::new(3).unwrap(), pane);
    host
}

#[test]
fn scope() {
    let mut host = host();
    let pane = Id::new(3).unwrap();
    let page = host.page(pane, &Read::default()).unwrap();
    assert_eq!((page.first, page.next, page.top), (0, 3, 0));
    let read = Read {
        scope: Some(page.scope),
        top: Some(0),
        ..Default::default()
    };
    assert!(host.page(pane, &read).is_ok());
    host.scope = [2; 16];
    assert_eq!(host.check(pane, &read), Err(Failure::Baseline));
    host.scope = [1; 16];
    host.panes
        .get_mut(&pane)
        .unwrap()
        .engine
        .resize(Extent::new(4, 3).unwrap())
        .unwrap();
    assert_eq!(host.check(pane, &read), Err(Failure::Baseline));
    let next = host.page(pane, &Read::default()).unwrap();
    let read = Read {
        scope: Some(next.scope),
        top: Some(0),
        ..Default::default()
    };
    host.panes.get_mut(&pane).unwrap().engine.trim(0);
    assert_eq!(host.check(pane, &read), Err(Failure::Baseline));
}

#[test]
fn budget() {
    let config = crate::config::History {
        lines: 50_000,
        bytes: 512,
        total: 1024,
    };
    let mut host = Host::configured([1; 16], config).unwrap();
    for value in 1..=8 {
        host.panes.insert(
            Id::new(value).unwrap(),
            Pane::new(Extent::new(80, 2).unwrap(), config).unwrap(),
        );
    }
    host.budget();
    for pane in host.panes.values_mut() {
        for _ in 0..100 {
            pane.engine.feed(b"row\r\n").unwrap();
        }
    }
    let bytes: usize = host
        .panes
        .values()
        .map(|pane| pane.engine.history().range().bytes)
        .sum();
    assert!(bytes <= config.total);
    assert!(
        host.panes
            .values()
            .all(|pane| pane.engine.history().range().evicted > 0)
    );
}

#[test]
fn partial() {
    let host = host();
    let page = host
        .page(
            Id::new(3).unwrap(),
            &Read {
                back: 1,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(page.top, 2);
    assert!(!page.screen.cursor.visible);
    assert_eq!(page.screen.cells[0].text, "t");
    assert_eq!(page.screen.cells[8].text, "t");
    assert_eq!(page.screen.cells.len(), 24);
    assert!(
        protocol::wire::encode(
            &Message::History {
                request: 1,
                state: page
            },
            protocol::wire::FRAME
        )
        .unwrap()
        .len()
            < 10_000
    );
}
