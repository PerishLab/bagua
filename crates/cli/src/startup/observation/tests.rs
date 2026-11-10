use crate::{args::Args, session::Session};
use clap::Parser;
use locus::{Atom, Hook, Observation};
use protocol::{
    Command, Failure, Reply, Request,
    wire::{Message, Program},
};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    os::unix::net::UnixListener,
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Default)]
struct Capture(Mutex<Vec<Atom>>);

impl Hook for Capture {
    fn observe(&self, observation: &Observation) {
        if let Observation::Report(outcome) = observation {
            self.0.lock().unwrap().push(outcome.atom().clone());
        }
    }
}

fn peer(listener: UnixListener) {
    for actor in 100..102 {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let hello = Message::Hello {
            version: protocol::VERSION,
            client: workspace::Id::new(actor).unwrap(),
            canvas: workspace::Geometry::new(80, 24).unwrap(),
            identity: None,
        };
        stream
            .write_all(&protocol::wire::encode(&hello, protocol::wire::FRAME).unwrap())
            .unwrap();
        let mut header = [0; 4];
        stream.read_exact(&mut header).unwrap();
        let length = u32::from_be_bytes(header) as usize;
        assert!(length <= protocol::wire::REQUEST);
        let mut bytes = vec![0; length];
        stream.read_exact(&mut bytes).unwrap();
        let request: Request = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(request.id, 1);
        assert!(matches!(request.command, Command::Start { .. }));
        let reply = Message::Reply(Reply {
            request: request.id,
            result: Err(Failure::Target),
        });
        stream
            .write_all(&protocol::wire::encode(&reply, protocol::wire::FRAME).unwrap())
            .unwrap();
    }
}

#[test]
fn refusal() {
    use plumb::observation::{Declaration, Run, Settings};
    let directory = tempfile::tempdir().unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = directory.path().join("host.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let capture = Arc::new(Capture::default());
    let run = Run::configured(
        Declaration {
            producer: "bagua",
            file: &directory.path().join("absent.toml"),
            buffer: directory.path().join("buffer"),
        },
        "control",
        Settings {
            enabled: true,
            endpoint: Some("http://127.0.0.1:1".into()),
        },
        capture.clone(),
    )
    .unwrap()
    .unwrap();
    let server = std::thread::spawn(move || peer(listener));
    let args = Args::parse_from(["bagua", "capabilities"]);
    let mut spans = Vec::new();
    for _ in 0..2 {
        let mut session = Session::connect(&path, &args).unwrap();
        spans.push(
            session
                .scope
                .as_ref()
                .unwrap()
                .read(&locus::Role::span())
                .unwrap()
                .text()
                .to_owned(),
        );
        let result = session.call(
            Command::Start {
                pane: workspace::Id::new(999).unwrap(),
                program: Program {
                    executable: "PRIVATE-PROGRAM-CANARY".into(),
                    args: vec!["PRIVATE-ARG-CANARY".into()],
                    cwd: "/PRIVATE-CWD-CANARY".into(),
                    environment: BTreeMap::from([("CANARY".into(), "PRIVATE-ENV-CANARY".into())]),
                },
            },
            false,
        );
        assert!(result.is_err());
    }
    server.join().unwrap();
    run.finish(2);
    assert_ne!(spans[0], spans[1]);
    let atoms = capture.0.lock().unwrap();
    for span in spans {
        let receipts: Vec<_> = atoms
            .iter()
            .filter(|atom| atom.context().get("locus.span") == Some(&span))
            .filter_map(Atom::payload)
            .filter(|payload| payload["event"] == "request")
            .collect();
        assert_eq!(receipts.len(), 2);
        assert_eq!(receipts[0]["request"], 1);
        assert_eq!(receipts[0]["stage"], "sent");
        assert_eq!(receipts[1]["stage"], "refused");
    }
    let bytes = serde_json::to_string(&*atoms).unwrap();
    for marker in [
        "PRIVATE-PROGRAM-CANARY",
        "PRIVATE-ARG-CANARY",
        "PRIVATE-CWD-CANARY",
        "PRIVATE-ENV-CANARY",
    ] {
        assert!(!bytes.contains(marker));
    }
}
