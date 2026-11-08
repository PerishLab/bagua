use super::{
    Arc, AtomicBool, Command, Connection, Duration, Fixture, Instant, Message, Request, fs,
};
use std::{
    io,
    os::unix::fs::{MetadataExt, PermissionsExt},
};

impl Fixture {
    pub fn cold() -> Self {
        let directory = tempfile::tempdir().unwrap();
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            directory: Some(directory),
            normal: true,
            cleanup: true,
            stop: Arc::new(AtomicBool::new(false)),
            task: None,
        }
    }

    pub fn foreign(&mut self) {
        self.cleanup = false;
    }

    pub fn shutdown(&mut self) -> bool {
        let completed = self.stop().is_ok();
        if completed {
            self.cleanup = false;
        }
        completed
    }

    fn stop(&self) -> io::Result<()> {
        let mut connection = Connection::connect(&self.path())?;
        connection.bounded(Duration::from_secs(2))?;
        let metadata = fs::metadata(env!("CARGO_BIN_EXE_bagua"))?;
        let identity = Some(protocol::wire::Image {
            device: metadata.dev(),
            inode: metadata.ino(),
        });
        match connection.receive()? {
            Message::Hello {
                version: protocol::VERSION,
                identity: observed,
                ..
            } if observed == identity => {}
            _ => return Err(io::Error::other("fixture endpoint identity is unknown")),
        }
        connection.send(&Request::new(Command::Shutdown))?;
        let mut completed = false;
        for _ in 0..64 {
            if matches!(
                connection.receive()?,
                Message::Finished {
                    request: 1,
                    result: Ok(())
                }
            ) {
                completed = true;
                break;
            }
        }
        if !completed {
            return Err(io::Error::other("fixture host did not finish shutdown"));
        }
        drop(connection);
        let deadline = Instant::now() + Duration::from_secs(3);
        while fs::symlink_metadata(self.path()).is_ok() {
            if Instant::now() >= deadline {
                return Err(io::Error::other("fixture endpoint remains"));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }

    pub(super) fn retain(&mut self) {
        if let Some(directory) = self.directory.take() {
            eprintln!(
                "owned fixture namespace retained: {}",
                directory.keep().display()
            );
        }
    }

    pub fn context(&self, pane: workspace::Id) -> protocol::wire::Context {
        let mut connection = Connection::connect(&self.path()).unwrap();
        connection.receive().unwrap();
        connection
            .send(&Request::new(Command::Workspace(
                workspace::Operation::Join {
                    session: workspace::Id::new(1).unwrap(),
                    viewport: workspace::Geometry::new(80, 24).unwrap(),
                },
            )))
            .unwrap();
        loop {
            if matches!(
                connection.receive().unwrap(),
                Message::Finished {
                    request: 1,
                    result: Ok(())
                }
            ) {
                break;
            }
        }
        connection
            .send(&Request::identified(2, Command::Describe(pane)).unwrap())
            .unwrap();
        loop {
            match connection.receive().unwrap() {
                Message::Context {
                    request: 2, state, ..
                } => return state,
                Message::Finished {
                    result: Err(fault), ..
                } => panic!("description failed: {fault:?}"),
                _ => {}
            }
        }
    }
}
