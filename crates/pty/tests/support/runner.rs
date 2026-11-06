use pty::{CHUNK, Exit, Interest, Launch, Process, Transfer};
use std::time::{Duration, Instant};

pub struct Runner {
    pub process: Process,
    pub output: Vec<u8>,
    eof: bool,
    deadline: Instant,
}

impl Runner {
    pub fn new(launch: Launch) -> Self {
        Self {
            process: Process::spawn(launch).unwrap(),
            output: Vec::new(),
            eof: false,
            deadline: Instant::now() + Duration::from_secs(10),
        }
    }

    fn check(&self) {
        assert!(
            Instant::now() < self.deadline,
            "native fixture deadline exceeded"
        );
    }

    fn pump(&mut self) {
        self.process.observe().unwrap();
        let mut bytes = [0; CHUNK];
        self.check();
        match self.process.read(&mut bytes).unwrap() {
            Transfer::Bytes(count) => self.output.extend_from_slice(&bytes[..count]),
            Transfer::Pending => {}
            Transfer::Eof => self.eof = true,
        }
        assert!(
            self.output.len() <= 2 * 1024 * 1024,
            "fixture output budget exceeded"
        );
    }

    pub fn until(&mut self, marker: &[u8]) {
        while !self
            .output
            .windows(marker.len())
            .any(|bytes| bytes == marker)
        {
            self.check();
            assert!(!self.eof, "fixture exited before marker");
            self.process
                .poll(Interest::Read, Duration::from_millis(10))
                .unwrap();
            self.pump();
        }
    }

    pub fn send(&mut self, mut bytes: &[u8]) {
        while !bytes.is_empty() {
            self.check();
            match self.process.write(bytes).unwrap() {
                Transfer::Bytes(count) => {
                    assert!(count > 0);
                    bytes = &bytes[count..];
                }
                Transfer::Pending => {
                    self.process
                        .poll(Interest::Write, Duration::from_millis(10))
                        .unwrap();
                }
                Transfer::Eof => panic!("fixture input closed"),
            }
        }
    }

    pub fn drain(&mut self) {
        while !self.eof {
            self.check();
            self.process
                .poll(Interest::Read, Duration::from_millis(10))
                .unwrap();
            self.pump();
        }
    }

    pub fn exit(&mut self) -> Exit {
        self.drain();
        loop {
            self.check();
            if let Some(exit) = self.process.observe().unwrap() {
                return exit;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
