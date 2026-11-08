use std::time::Instant;

use terminal::{Engine, Extent, Failure, HISTORY, PAGE, Point};

#[test]
fn narrow() {
    exercise(80);
}

#[test]
fn wide() {
    exercise(160);
}

fn exercise(width: u16) {
    let begin = Instant::now();
    let mut engine = Engine::new(Extent::new(width, 24).unwrap(), HISTORY).unwrap();
    let padding = "x".repeat(usize::from(width) - 6);
    for index in 0..50_023 {
        engine
            .feed(format!("{index:05} {padding}\r\n").as_bytes())
            .unwrap();
    }
    let range = engine.history().range();
    assert_eq!(engine.history().count(), 50_000);
    assert_eq!((range.first, range.next, range.evicted), (0, 50_000, 0));
    for first in (0..50_000).step_by(PAGE) {
        for line in engine
            .history()
            .page(first, (50_000 - first).min(PAGE as u64) as usize)
            .unwrap()
        {
            assert_eq!(line.text(), format!("{:05} {padding}", line.id));
            assert_eq!(line.columns, width);
            assert!(!line.wrapped);
        }
    }
    let snapshot = engine.snapshot();
    assert!(
        snapshot
            .view(50_000)
            .unwrap()
            .contents()
            .starts_with("00000 ")
    );
    let elapsed = begin.elapsed();
    let clones = Instant::now();
    for _ in 0..1000 {
        std::hint::black_box(engine.snapshot());
    }
    eprintln!(
        "width={width} rows={} retained_bytes={} populate_read_ms={} snapshot1000_ms={}",
        engine.history().count(),
        range.bytes,
        elapsed.as_millis(),
        clones.elapsed().as_millis()
    );
    engine.feed(b"last\r\n").unwrap();
    assert_eq!(engine.history().range().first, 1);
    assert_eq!(engine.history().range().evicted, 1);
    assert!(matches!(
        engine.history().page(0, 1),
        Err(Failure::Position)
    ));
    assert!(
        snapshot
            .view(50_000)
            .unwrap()
            .contents()
            .starts_with("00000 ")
    );
}

#[test]
fn geometry() {
    let mut engine = Engine::new(Extent::new(8, 2).unwrap(), 20).unwrap();
    engine.feed(b"abcdefghZ\r\nnext\r\n").unwrap();
    let rows = engine.history().page(0, 2).unwrap();
    assert_eq!(rows[0].text(), "abcdefgh");
    assert!(rows[0].wrapped);
    assert_eq!(rows[1].text(), "Z");
    engine.resize(Extent::new(4, 2).unwrap()).unwrap();
    assert_eq!(engine.history().page(0, 1).unwrap()[0].text(), "abcdefgh");
    engine.resize(Extent::new(12, 2).unwrap()).unwrap();
    let snapshot = engine.snapshot().view(engine.history().count()).unwrap();
    assert_eq!(snapshot.cell(Point { x: 10, y: 0 }).unwrap().text, "");
    assert!(snapshot.contents().starts_with("abcdefghZ"));
}

#[test]
fn clearing() {
    let mut engine = Engine::new(Extent::new(8, 2).unwrap(), 20).unwrap();
    engine.feed(b"one\r\ntwo\r\n").unwrap();
    engine.feed(b"\x1b]2;literal [3J\x07").unwrap();
    assert_eq!(engine.history().count(), 1);
    engine.feed(b"\x1b[?1049hA\r\nB\r\nC\x1b[3J").unwrap();
    assert_eq!(engine.history().count(), 1);
    assert_eq!(engine.snapshot().history(), 0);
    engine.feed(b"\x1b[?1049l\x1b[3").unwrap();
    assert_eq!(engine.history().count(), 1);
    engine.feed(b"Jnew\r\n").unwrap();
    let range = engine.history().range();
    assert_eq!((range.first, range.next, range.evicted), (1, 2, 1));
    engine.feed(b"\x1bc").unwrap();
    assert_eq!(engine.history().count(), 0);
    assert_eq!(engine.history().range().next, 2);
}

#[test]
fn budget() {
    let mut engine = Engine::new(Extent::new(80, 2).unwrap(), HISTORY).unwrap();
    for _ in 0..100 {
        engine.feed(b"value\r\n").unwrap();
    }
    engine.trim(1000);
    assert!(engine.history().range().bytes <= 1000);
    assert!(engine.history().count() < 100);
    assert!(engine.history().range().evicted > 0);
    assert!(
        engine
            .history()
            .page(engine.history().range().first, PAGE + 1)
            .is_err()
    );
    engine.trim(0);
    assert!(engine.history().empty());
}

#[test]
fn parameters() {
    for sequence in [b"\x1b[3;1J".as_slice(), b"\x1b[3:1J".as_slice()] {
        let mut engine = Engine::new(Extent::new(8, 2).unwrap(), 20).unwrap();
        engine.feed(b"one\r\ntwo\r\n").unwrap();
        let epoch = engine.epoch();
        engine.feed(sequence).unwrap();
        assert!(engine.history().empty());
        assert!(engine.epoch() > epoch);
    }
}
