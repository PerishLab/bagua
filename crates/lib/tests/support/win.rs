use bagua::Host;
use protocol::{
    Command, Failure, Request,
    wire::{Message, Program, Screen},
};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{Duration, Instant},
};
use workspace::{Geometry, Id, Operation, Target};

pub fn id(value: u64) -> Id {
    Id::new(value).unwrap()
}

pub fn powershell() -> String {
    let root = std::env::var("SystemRoot")
        .or_else(|_| std::env::var("WINDIR"))
        .unwrap_or_else(|_| r"C:\Windows".into());
    PathBuf::from(root)
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe")
        .to_string_lossy()
        .into_owned()
}

pub fn program(script: &str) -> Program {
    let mut environment = BTreeMap::new();
    for key in [
        "SYSTEMROOT",
        "WINDIR",
        "COMSPEC",
        "PATH",
        "PATHEXT",
        "TEMP",
        "TMP",
    ] {
        if let Ok(value) = std::env::var(key) {
            environment.insert(key.into(), value);
        }
    }
    environment.insert("TERM".into(), "xterm-256color".into());
    Program {
        executable: powershell(),
        args: vec![
            "-NoLogo".into(),
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-Command".into(),
            script.into(),
        ],
        cwd: std::env::temp_dir().to_string_lossy().into_owned(),
        environment,
    }
}

pub fn text(screen: &Screen) -> String {
    screen
        .cells
        .iter()
        .map(|cell| {
            if cell.text.is_empty() {
                " "
            } else {
                cell.text.as_str()
            }
        })
        .collect()
}

pub struct Driver {
    pub host: Host,
    pub actor: Id,
    pub messages: Vec<Message>,
    next: u64,
}

impl Driver {
    pub fn new() -> Self {
        let mut host = Host::new();
        let actor = host.connect().unwrap();
        Self {
            host,
            actor,
            messages: Vec::new(),
            next: 1,
        }
    }

    pub fn submit(&mut self, command: Command) -> u64 {
        let request = self.next;
        self.next += 1;
        self.host
            .submit(self.actor, Request::identified(request, command).unwrap())
            .unwrap();
        request
    }

    pub fn call(&mut self, command: Command) -> Result<u64, Failure> {
        let request = self.submit(command);
        self.until(|driver| {
            driver.messages.iter().any(|message| match message {
                Message::Finished { request: id, .. } => *id == request,
                Message::Reply(reply) => reply.request == request && reply.result.is_err(),
                _ => false,
            })
        });
        for message in &self.messages {
            match message {
                Message::Finished {
                    request: id,
                    result: Err(_),
                } if *id == request => return Err(Failure::Native),
                Message::Reply(reply) if reply.request == request => {
                    reply.result.clone()?;
                }
                _ => {}
            }
        }
        Ok(request)
    }

    pub fn drive(&mut self) {
        self.host.tick().unwrap();
        while let Some(bytes) = self.host.output(self.actor) {
            let length = bytes.len();
            self.messages
                .push(serde_json::from_slice(&bytes[4..]).unwrap());
            self.host.consume(self.actor, length);
        }
    }

    pub fn until(&mut self, mut done: impl FnMut(&Self) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(15);
        while !done(self) {
            assert!(
                Instant::now() < deadline,
                "host did not make expected progress"
            );
            self.drive();
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    pub fn create(&mut self) {
        self.call(Command::create(80, 24).unwrap()).unwrap();
        self.join();
        self.call(Command::Workspace(Operation::Acquire {
            session: id(1),
            fixed: None,
        }))
        .unwrap();
    }

    pub fn join(&mut self) {
        self.call(Command::Workspace(Operation::Join {
            session: id(1),
            viewport: Geometry::new(80, 24).unwrap(),
        }))
        .unwrap();
    }

    pub fn start(&mut self, script: &str) {
        self.call(Command::Start {
            pane: id(3),
            program: program(script),
        })
        .unwrap();
        self.call(Command::Subscribe {
            target: Target::Pane(id(3)),
            after: None,
        })
        .unwrap();
    }

    pub fn contains(&self, value: &str) -> bool {
        self.messages
            .iter()
            .any(|message| matches!(message, Message::Screen { state, .. } if text(state).contains(value)))
    }
}
