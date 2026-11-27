use super::peer;
use bagua::ipc::{Connection, Service};
use protocol::{Command, Request, wire::Message};
use pty::Size;
use std::{
    fs,
    path::Path,
    process::Command as Exec,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use workspace::{Geometry, Operation};

fn seed(socket: &Path, name: &str) -> (Arc<AtomicBool>, std::thread::JoinHandle<()>) {
    let mut service = Service::bind(socket).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    let task = std::thread::spawn(move || service.run(&flag).unwrap());
    let mut client = Connection::connect(socket).unwrap();
    client.receive().unwrap();
    client
        .send(&Request::new(Command::Workspace(Operation::Named {
            geometry: Geometry::new(80, 24).unwrap(),
            name: name.into(),
        })))
        .unwrap();
    loop {
        if let Message::Finished { result: Ok(()), .. } = client.receive().unwrap() {
            break;
        }
    }
    (stop, task)
}

#[test]
fn discovery() {
    let home = tempfile::tempdir().unwrap();
    let runtime = home.path().join("bagua").join("runtime");
    fs::create_dir_all(&runtime).unwrap();
    bagua::ipc::secure(&runtime).unwrap();
    let default = runtime.join("host.sock");

    let extra = peer::directory();
    let envsocket = extra.path().join("env.sock");
    let clisocket = extra.path().join("cli.sock");

    let (stopa, taska) = seed(&default, "default");
    let (stopb, taskb) = seed(&envsocket, "env");
    let (stopc, taskc) = seed(&clisocket, "cli");

    let query = |extraenv: Option<&Path>, explicit: Option<&Path>| -> String {
        let mut command = Exec::new(env!("CARGO_BIN_EXE_bagua"));
        command.env_clear();
        command
            .env("LOCALAPPDATA", home.path())
            .env("USERPROFILE", home.path())
            .env("SYSTEMROOT", peer::systemroot());
        if let Some(socket) = extraenv {
            command.env("BAGUA_SOCKET", socket);
        }
        command.arg("--no-config").arg("--json");
        if let Some(socket) = explicit {
            command.arg("--socket").arg(socket);
        }
        command.arg("list");
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let body = String::from_utf8(output.stdout).unwrap();
        let message: serde_json::Value =
            serde_json::from_str(body.lines().next().unwrap()).unwrap();
        message["Reply"]["result"]["Ok"]["Applied"]["Workspace"]["Snapshot"]["sessions"][0]["name"]
            .as_str()
            .unwrap()
            .to_owned()
    };

    assert_eq!(query(None, None), "default");
    assert_eq!(query(Some(&envsocket), None), "env");
    assert_eq!(query(Some(&envsocket), Some(&clisocket)), "cli");

    stopa.store(true, Ordering::Relaxed);
    stopb.store(true, Ordering::Relaxed);
    stopc.store(true, Ordering::Relaxed);
    taska.join().unwrap();
    taskb.join().unwrap();
    taskc.join().unwrap();
}

#[test]
fn awareness() {
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join("bagua");
    fs::create_dir_all(&config).unwrap();
    fs::write(
        config.join("bagua.toml"),
        "[header]\nenabled=false\n[status]\ncomponents=['session','windows','prefix','notice']\n",
    )
    .unwrap();
    let socket = home.path().join("bagua").join("runtime").join("host.sock");
    let mut peer = peer::Peer::spawn(home.path(), &["enter", "work"], Size::new(120, 30).unwrap());
    peer.until("control");
    let expected = fs::canonicalize(socket.parent().unwrap())
        .unwrap()
        .join("host.sock");
    let expected = expected
        .to_str()
        .unwrap()
        .strip_prefix(r"\\?\")
        .unwrap_or_else(|| expected.to_str().unwrap());
    peer.send(
        b"Write-Output \"P1=$($env:BAGUA_SOCKET)|$($env:BAGUA_SESSION)|$($env:BAGUA_WINDOW)|$($env:BAGUA_PANE)\"\r",
    );
    peer.until(&format!("P1={expected}|1|2|3"));
    let exe = env!("CARGO_BIN_EXE_bagua");
    peer.send(format!("& '{exe}' --no-config --json list\r").as_bytes());
    peer.until("\"name\":\"work\"");
    peer.send(b"\x07-");
    let deadline = Instant::now() + Duration::from_secs(15);
    while peer::catalog(&socket)
        .sessions()
        .flat_map(|session| session.windows())
        .map(|window| window.panes().count())
        .sum::<usize>()
        != 2
    {
        assert!(Instant::now() < deadline);
        peer.pump();
        std::thread::sleep(Duration::from_millis(10));
    }
    peer.send(
        b"Write-Output \"P2=$($env:BAGUA_SOCKET)|$($env:BAGUA_SESSION)|$($env:BAGUA_WINDOW)|$($env:BAGUA_PANE)\"\r",
    );
    peer.until(&format!("P2={expected}|1|2|4"));
    peer.send(b"\x07=");
    let deadline = Instant::now() + Duration::from_secs(15);
    while peer::catalog(&socket)
        .sessions()
        .next()
        .unwrap()
        .windows()
        .count()
        != 2
    {
        assert!(Instant::now() < deadline);
        peer.pump();
        std::thread::sleep(Duration::from_millis(10));
    }
    peer.send(
        b"Write-Output \"P3=$($env:BAGUA_SOCKET)|$($env:BAGUA_SESSION)|$($env:BAGUA_WINDOW)|$($env:BAGUA_PANE)\"\r",
    );
    peer.until(&format!("P3={expected}|1|5|6"));
    peer.send(b"\x07d");
    assert_eq!(peer.finish(), 0);
    peer::shutdown(&socket);
}
