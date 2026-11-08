use bagua::ipc::{Connection, Service};
#[path = "interactive/startup.rs"]
mod startup;
#[path = "interactive/view.rs"]
mod view;
use protocol::{Command, Request, wire::Message};
use pty::{Launch, Process, Size, Transfer};
use radicula_process::Environment;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub struct Fixture {
    directory: Option<tempfile::TempDir>,
    normal: bool,
    cleanup: bool,
    stop: Arc<AtomicBool>,
    task: Option<std::thread::JoinHandle<()>>,
}

impl Fixture {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut service = Service::bind(&directory.path().join("host.sock")).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let task = std::thread::spawn(move || service.run(&flag).unwrap());
        Self {
            directory: Some(directory),
            normal: false,
            cleanup: false,
            stop,
            task: Some(task),
        }
    }
    pub fn path(&self) -> PathBuf {
        if self.normal {
            self.home().join(".local/state/bagua/host.sock")
        } else {
            self.home().join("host.sock")
        }
    }
    fn home(&self) -> &std::path::Path {
        self.directory.as_ref().unwrap().path()
    }
    pub fn catalog(&self) -> workspace::Snapshot {
        let mut connection = Connection::connect(&self.path()).unwrap();
        connection.receive().unwrap();
        connection
            .send(&Request::new(Command::Workspace(
                workspace::Operation::Read,
            )))
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
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.cleanup && !self.shutdown() {
            self.retain();
            assert!(
                std::thread::panicking(),
                "owned host cleanup is unknown; namespace retained"
            );
            return;
        }
        self.stop.store(true, Ordering::Relaxed);
        let failed = self.task.take().is_some_and(|task| task.join().is_err());
        assert!(!failed || std::thread::panicking(), "service failed");
    }
}

pub struct Peer {
    pub process: Process,
    pub engine: terminal::Engine,
    pub output: Vec<u8>,
}

impl Peer {
    pub fn new(fixture: &Fixture, args: &[&str], size: Size) -> Self {
        Self::spawn(fixture, args, size, true)
    }
    pub fn ordinary(fixture: &Fixture, args: &[&str], size: Size) -> Self {
        Self::spawn(fixture, args, size, false)
    }
    fn spawn(fixture: &Fixture, args: &[&str], size: Size, socket: bool) -> Self {
        let script = "before=$(stty -g); \"$@\"; status=$?; after=$(stty -g); if [ \"$before\" = \"$after\" ]; then printf '\\nRESTORED:%s\\n' \"$status\"; else printf '\\nBROKEN-MODE\\n'; fi";
        let mut launch = Launch::new(
            "/bin/sh".into(),
            "/".into(),
            Environment::new([
                ("PATH".into(), "/usr/bin:/bin".into()),
                ("TERM".into(), "xterm-256color".into()),
                ("HOME".into(), fixture.home().into()),
                ("LC_ALL".into(), "C.UTF-8".into()),
            ]),
            size,
        );
        launch.args = vec![
            "-c".into(),
            script.into(),
            "bagua-fixture".into(),
            env!("CARGO_BIN_EXE_bagua").into(),
        ];
        if socket {
            launch
                .args
                .extend(["--socket".into(), fixture.path().into()]);
        }
        launch.args.extend(args.iter().map(|arg| (*arg).into()));
        Self {
            process: Process::spawn(launch).unwrap(),
            engine: terminal::Engine::new(
                terminal::Extent::new(size.columns(), size.rows()).unwrap(),
                0,
            )
            .unwrap(),
            output: Vec::new(),
        }
    }
    pub fn text(&self) -> String {
        self.engine.snapshot().contents()
    }
    pub fn pump(&mut self) {
        let mut bytes = [0; pty::CHUNK];
        if let Transfer::Bytes(count) = self.process.read(&mut bytes).unwrap() {
            self.engine.feed(&bytes[..count]).unwrap();
            self.output.extend_from_slice(&bytes[..count]);
        }
        assert!(
            self.output.len() < 16_777_216,
            "fixture output exceeded budget"
        );
        self.process.observe().unwrap();
    }
    pub fn until(&mut self, marker: &str) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !self.text().contains(marker) {
            assert!(
                Instant::now() < deadline,
                "missing {marker}; screen: {}",
                self.text()
            );
            self.pump();
            std::thread::sleep(Duration::from_millis(2));
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
    pub fn detach(&mut self) {
        self.send(b"\x07d");
        self.until("RESTORED:0");
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.process.observe().unwrap().is_none() {
            assert!(Instant::now() < deadline);
            self.pump();
        }
    }
}

pub const SHELL: &[&str] = &[
    "open",
    "--executable",
    "/bin/sh",
    "--cwd",
    "/",
    "--arg=-i",
    "--env",
    "PATH=/usr/bin:/bin",
    "--env",
    "TERM=xterm-256color",
];
