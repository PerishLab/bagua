use protocol::{Command, Failure, Request};

#[test]
fn unavailable() {
    for command in [
        Command::create(80, 24).unwrap(),
        Command::Workspace(workspace::Operation::Read),
        Command::attach(1, 80, 24).unwrap(),
    ] {
        assert_eq!(
            bagua::dispatch(Request::new(command)),
            Err(Failure::Unavailable)
        );
    }
    let request = Request {
        version: 0,
        id: 1,
        command: Command::create(80, 24).unwrap(),
    };
    assert_eq!(bagua::dispatch(request), Err(Failure::Version));
}

#[test]
fn profile() {
    let profile = bagua::dispatch(Request::new(Command::Capabilities)).unwrap();
    assert_eq!(profile.runtime, pty::SUPPORTED);
    assert_eq!(profile.components.len(), 7);
    assert_eq!(
        profile
            .components
            .iter()
            .filter(|c| c.state == protocol::State::Available)
            .count(),
        if pty::SUPPORTED { 7 } else { 1 }
    );
    assert!(
        profile
            .components
            .iter()
            .all(|c| c.state != protocol::State::Unverified)
    );
}

#[test]
fn observation() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("bagua.toml");
    let load = || bagua::config::Config::resolve(Some(&path), "BAGUA", &|_| None);
    std::fs::write(&path, "[locus]\nenabled = 'invalid'\nendpoint = 42\n").unwrap();
    let config = load().unwrap();
    assert_eq!(config.locus.unwrap()["enabled"], "invalid");
    assert!(bagua::config::Config::default().locus.is_none());
    std::fs::write(&path, "[future]\nenabled = true\n").unwrap();
    assert!(load().is_err());
}

#[cfg(any(
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "windows", target_arch = "x86_64")
))]
mod injected {
    use bagua::ipc::{Connection, Service};
    use protocol::{
        Command, Request,
        wire::{Message, Program},
    };
    use std::{
        collections::BTreeMap,
        path::Path,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, Instant},
    };
    use workspace::{Axis, Geometry, Id, Operation, Ratio, Split};

    fn id(value: u64) -> Id {
        Id::new(value).unwrap()
    }

    fn directory() -> (tempfile::TempDir, std::path::PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let root = {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700))
                .unwrap();
            std::fs::canonicalize(directory.path()).unwrap()
        };
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        let root = {
            bagua::ipc::secure(directory.path()).unwrap();
            directory.path().to_path_buf()
        };
        (directory, root)
    }

    fn program(cwd: &Path, tag: &str) -> Program {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            Program {
                executable: "/bin/sh".into(),
                args: vec![
                    "-c".into(),
                    format!(
                        "printf '{tag}=%s|%s|%s|%s|%s\\n' \"$USER_KEEP\" \"$BAGUA_SOCKET\" \"$BAGUA_SESSION\" \"$BAGUA_WINDOW\" \"$BAGUA_PANE\"; exec /bin/sleep 60"
                    ),
                ],
                cwd: cwd.to_string_lossy().into_owned(),
                environment: BTreeMap::from([
                    ("PATH".into(), "/usr/bin:/bin".into()),
                    ("USER_KEEP".into(), "kept".into()),
                    ("BAGUA_SOCKET".into(), "stale".into()),
                    ("BAGUA_SESSION".into(), "99".into()),
                    ("BAGUA_WINDOW".into(), "99".into()),
                    ("BAGUA_PANE".into(), "99".into()),
                ]),
            }
        }
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            let root = std::env::var("SYSTEMROOT").unwrap_or_else(|_| r"C:\Windows".into());
            let comspec = Path::new(&root)
                .join("System32")
                .join("cmd.exe")
                .to_string_lossy()
                .into_owned();
            Program {
                executable: comspec,
                args: vec![
                    "/Q".into(),
                    "/K".into(),
                    format!(
                        "echo {tag}=%USER_KEEP%^|%BAGUA_SOCKET%^|%BAGUA_SESSION%^|%BAGUA_WINDOW%^|%BAGUA_PANE%"
                    ),
                ],
                cwd: cwd.to_string_lossy().into_owned(),
                environment: BTreeMap::from([
                    ("SYSTEMROOT".into(), root),
                    ("USER_KEEP".into(), "kept".into()),
                    ("BAGUA_SOCKET".into(), "stale".into()),
                    ("bagua_socket".into(), "stale2".into()),
                    ("BAGUA_SESSION".into(), "99".into()),
                    ("BAGUA_WINDOW".into(), "99".into()),
                    ("BAGUA_PANE".into(), "99".into()),
                ]),
            }
        }
    }

    fn call(client: &mut Connection, next: &mut u64, command: Command) -> Vec<Message> {
        let requestid = *next;
        *next += 1;
        client
            .send(&Request::identified(requestid, command).unwrap())
            .unwrap();
        let mut out = Vec::new();
        loop {
            let message = client.receive().unwrap();
            let done = matches!(&message, Message::Finished { request, result } if *request == requestid && result.is_ok());
            assert!(
                !matches!(&message, Message::Reply(reply) if reply.request == requestid && reply.result.is_err())
            );
            assert!(
                !matches!(&message, Message::Finished { request, result: Err(_) } if *request == requestid)
            );
            out.push(message);
            if done {
                return out;
            }
        }
    }

    fn render(messages: Vec<Message>) -> String {
        let mut text = String::new();
        for message in messages {
            let Message::Screen { state, .. } = message else {
                continue;
            };
            for cell in state.cells {
                if cell.text.is_empty() {
                    text.push(' ');
                } else {
                    text.push_str(&cell.text);
                }
            }
        }
        text
    }

    fn wait(client: &mut Connection, next: &mut u64, pane: Id, marker: &str) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let text = render(call(client, next, Command::Inspect(pane)));
            if text.contains(marker) {
                return;
            }
            assert!(Instant::now() < deadline, "missing {marker} in {text:?}");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn environment() {
        let (_guard, root) = directory();
        let path = root.join("host.sock");
        let mut service = Service::bind(&path).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let task = std::thread::spawn(move || service.run(&flag).unwrap());
        let mut client = Connection::connect(&path).unwrap();
        client.receive().unwrap();
        let mut next = 1u64;
        call(&mut client, &mut next, Command::create(80, 24).unwrap());
        call(
            &mut client,
            &mut next,
            Command::Workspace(Operation::Join {
                session: id(1),
                viewport: Geometry::new(80, 24).unwrap(),
            }),
        );
        call(
            &mut client,
            &mut next,
            Command::Workspace(Operation::Acquire {
                session: id(1),
                fixed: None,
            }),
        );
        let expected = path.to_string_lossy().into_owned();
        call(
            &mut client,
            &mut next,
            Command::Start {
                pane: id(3),
                program: program(&root, "P1"),
            },
        );
        wait(
            &mut client,
            &mut next,
            id(3),
            &format!("P1=kept|{expected}|1|2|3"),
        );
        call(
            &mut client,
            &mut next,
            Command::Workspace(Operation::Split {
                pane: id(3),
                split: Split {
                    axis: Axis::Columns,
                    ratio: Ratio::new(500).unwrap(),
                },
            }),
        );
        call(
            &mut client,
            &mut next,
            Command::Start {
                pane: id(4),
                program: program(&root, "P2"),
            },
        );
        wait(
            &mut client,
            &mut next,
            id(4),
            &format!("P2=kept|{expected}|1|2|4"),
        );
        call(
            &mut client,
            &mut next,
            Command::Workspace(Operation::Window(id(1))),
        );
        call(
            &mut client,
            &mut next,
            Command::Start {
                pane: id(6),
                program: program(&root, "P3"),
            },
        );
        wait(
            &mut client,
            &mut next,
            id(6),
            &format!("P3=kept|{expected}|1|5|6"),
        );
        stop.store(true, Ordering::Relaxed);
        task.join().unwrap();
    }
}
