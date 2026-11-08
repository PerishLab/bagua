use super::*;

#[derive(Default)]
struct Slow {
    bytes: Vec<u8>,
    blocked: bool,
}

impl Write for Slow {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.blocked = !self.blocked;
        if self.blocked {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let count = bytes.len().min(1);
        self.bytes.extend_from_slice(&bytes[..count]);
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn framing() {
    let mut output = Output::default();
    let mut writer = Slow::default();
    let frame = "\x1b[2J中\x1b[0m".as_bytes();
    output.offer(frame.into()).unwrap();
    output.flush(&mut writer).unwrap();
    output.flush(&mut writer).unwrap();
    assert_eq!(writer.bytes, b"\x1b");
    output.offer(b"old".to_vec()).unwrap();
    assert!(output.copy(&Copy(b"\x1b]52;c;QQ==\x1b\\".to_vec())));
    output.offer(b"new".to_vec()).unwrap();
    for _ in 0..200 {
        output.flush(&mut writer).unwrap();
    }
    assert_eq!(
        writer.bytes,
        [frame, b"\x1b]52;c;QQ==\x1b\\", b"new"].concat()
    );
    assert_eq!(output.count, 0);
    assert_eq!(output.bytes, 0);
}

#[test]
fn custody() {
    let mut output = Output::default();
    let copy = Copy(vec![b'x'; 524_288]);
    for _ in 0..4 {
        assert!(output.copy(&copy));
    }
    let mut writer = Slow::default();
    output.flush(&mut writer).unwrap();
    output.flush(&mut writer).unwrap();
    assert!(!output.copy(&Copy(vec![b'y'])));
    assert_eq!(output.count, 4);
    assert_eq!(output.bytes, 2_097_152);
    let mut bytes = Vec::new();
    output.offer(b"droppable".to_vec()).unwrap();
    output.drain(&mut bytes).unwrap();
    assert_eq!(bytes.len(), 2_097_151);
    assert!(bytes.iter().all(|byte| *byte == b'x'));
    assert_eq!(output.count, 0);
}

#[test]
fn failure() {
    let mut output = Output::default();
    assert!(output.copy(&Copy(b"reliable".to_vec())));
    let mut storage = [];
    let mut closed = &mut storage[..];
    assert!(output.flush(&mut closed).is_err());
    assert_eq!(output.count, 1);
    assert_eq!(output.bytes, 8);
    let mut bytes = Vec::new();
    output.drain(&mut bytes).unwrap();
    assert_eq!(bytes, b"reliable");
}
fn paint(text: &str, bar: &str) -> Paint {
    Paint {
        rows: vec![
            format!("\x1b[0m\x1b[1;1H\x1b[2K{text:12}\x1b[0m"),
            format!("\x1b[0m\x1b[2;1H\x1b[2K{bar:12}\x1b[0m"),
        ],
        cursor: None,
        extent: terminal::Extent::new(12, 2).unwrap(),
    }
}

#[test]
fn baseline() {
    let mut output = Output::default();
    let mut writer = Slow::default();
    let first = paint("A", "X").render(None);
    output.present(paint("A", "X")).unwrap();
    output.flush(&mut writer).unwrap();
    output.flush(&mut writer).unwrap();
    assert!(output.baseline.is_none());
    output.present(paint("B", "X")).unwrap();
    let effect = b"\x1b]52;c;QQ==\x1b\\";
    assert!(output.copy(&Copy(effect.to_vec())));
    output.present(paint("B", "Y")).unwrap();
    for _ in 0..2000 {
        output.flush(&mut writer).unwrap();
    }
    assert_eq!(&writer.bytes[..first.len()], first);
    assert_eq!(
        &writer.bytes[first.len()..first.len() + effect.len()],
        effect
    );
    let mut engine = terminal::Engine::new(terminal::Extent::new(12, 2).unwrap(), 0).unwrap();
    for chunk in writer.bytes.chunks(terminal::CHUNK) {
        engine.feed(chunk).unwrap();
    }
    assert_eq!(
        engine
            .snapshot()
            .cell(terminal::Point { x: 0, y: 0 })
            .unwrap()
            .text,
        "B"
    );
    assert_eq!(
        engine
            .snapshot()
            .cell(terminal::Point { x: 0, y: 1 })
            .unwrap()
            .text,
        "Y"
    );
    output.present(paint("B", "Y")).unwrap();
    let mut bytes = Vec::new();
    output.flush(&mut bytes).unwrap();
    assert!(bytes.is_empty());
    output.present(paint("B", "Z")).unwrap();
    output.flush(&mut bytes).unwrap();
    assert_eq!(
        bytes
            .windows(4)
            .filter(|bytes| *bytes == b"\x1b[2K")
            .count(),
        1
    );
    assert!(!bytes.windows(4).any(|bytes| bytes == b"\x1b[2J"));
}
