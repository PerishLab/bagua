use bagua::ipc::Connection;
use protocol::{Command, Request, wire::Message};
use pty::{Launch, Process, Size, Transfer};
use radicula_process::Environment;
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use workspace::Operation;

pub fn directory() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    bagua::ipc::secure(directory.path()).unwrap();
    directory
}

pub fn systemroot() -> PathBuf {
    std::env::var_os("SYSTEMROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
}

pub fn comspec() -> String {
    systemroot()
        .join("System32")
        .join("cmd.exe")
        .to_string_lossy()
        .into_owned()
}

pub fn catalog(path: &Path) -> workspace::Snapshot {
    let mut connection = Connection::connect(path).unwrap();
    connection.receive().unwrap();
    connection
        .send(&Request::new(Command::Workspace(Operation::Read)))
        .unwrap();
    let snapshot = match connection.receive().unwrap() {
        Message::Reply(protocol::Reply {
            result:
                Ok(protocol::Status::Applied(protocol::Outcome::Workspace(
                    workspace::Outcome::Snapshot(value),
                ))),
            ..
        }) => value,
        other => panic!("unexpected catalog: {other:?}"),
    };
    assert!(matches!(
        connection.receive().unwrap(),
        Message::Finished { result: Ok(()), .. }
    ));
    snapshot
}

pub fn shutdown(path: &Path) {
    if let Ok(mut connection) = Connection::connect(path) {
        let _ = connection.receive();
        let _ = connection.send(&Request::new(Command::Shutdown));
        let _ = connection.receive();
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while path.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub struct Peer {
    process: Process,
    engine: terminal::Engine,
}

impl Peer {
    pub fn spawn(home: &Path, args: &[&str], size: Size) -> Self {
        let mut values = Vec::new();
        for key in [
            "SYSTEMROOT",
            "WINDIR",
            "COMSPEC",
            "PATH",
            "PATHEXT",
            "TEMP",
            "TMP",
        ] {
            if let Some(value) = std::env::var_os(key) {
                values.push((key.into(), value));
            }
        }
        values.push(("LOCALAPPDATA".into(), home.as_os_str().into()));
        values.push(("USERPROFILE".into(), home.as_os_str().into()));
        let mut launch = Launch::new(
            PathBuf::from(env!("CARGO_BIN_EXE_bagua")),
            home.to_path_buf(),
            Environment::new(values),
            size,
        );
        launch.args = args.iter().map(|arg| (*arg).into()).collect();
        Self {
            process: Process::spawn(launch).unwrap(),
            engine: terminal::Engine::new(
                terminal::Extent::new(size.columns(), size.rows()).unwrap(),
                0,
            )
            .unwrap(),
        }
    }

    pub fn text(&self) -> String {
        self.engine.snapshot().contents()
    }

    pub fn pump(&mut self) {
        let mut bytes = [0; pty::CHUNK];
        if let Transfer::Bytes(count) = self.process.read(&mut bytes).unwrap() {
            self.engine.feed(&bytes[..count]).unwrap();
        }
        self.process.observe().unwrap();
    }

    pub fn until(&mut self, marker: &str) {
        let deadline = Instant::now() + Duration::from_secs(15);
        while !self.text().contains(marker) {
            assert!(
                Instant::now() < deadline,
                "missing {marker}; screen: {}",
                self.text()
            );
            self.pump();
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    pub fn send(&mut self, mut bytes: &[u8]) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !bytes.is_empty() {
            assert!(Instant::now() < deadline);
            match self.process.write(bytes).unwrap() {
                Transfer::Bytes(count) => bytes = &bytes[count..],
                Transfer::Pending => self.pump(),
                Transfer::Eof => panic!("interactive client exited"),
            }
        }
    }

    pub fn finish(&mut self) -> i32 {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(exit) = self.process.observe().unwrap() {
                return exit.code.unwrap_or(-1);
            }
            assert!(Instant::now() < deadline, "client did not exit");
            self.pump();
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
