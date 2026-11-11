mod bars;
use super::interactive::{Fixture, Peer};
use pty::Size;
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

const CONFIG: &str = "[client]\nprefix = 'C-g'\n[header]\nenabled = true\ncomponents=['pane','command','cwd']\n[status]\ncomponents=['session','windows','prefix','notice','clock']\nseparator = true\ntimezone = 'Asia/Shanghai'\nrefresh_seconds = 1\n[program]\nexecutable='/bin/sh'\nargs=['-i']\ncwd='/'\nenvironment={PATH='/usr/bin:/bin',TERM='xterm-256color'}\n";

fn configured(fixture: &Fixture, path: &Path, size: Size) -> Peer {
    fs::write(path, CONFIG).unwrap();
    Peer::new(fixture, &["--config", path.to_str().unwrap(), "open"], size)
}

fn budget(fixture: &Fixture, peer: &mut Peer, header: bool, rows: u16) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let catalog = fixture.catalog();
        let session = catalog.sessions().next().unwrap();
        if session.header() == header && session.geometry().rows() == rows {
            break;
        }
        assert!(Instant::now() < deadline, "header/canvas adoption missing");
        peer.pump();
    }
}

#[test]
fn context() {
    let fixture = Fixture::new();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("bagua.toml");
    let mut peer = configured(&fixture, &path, Size::new(100, 24).unwrap());
    peer.until("cwd~/");
    budget(&fixture, &mut peer, true, 22);
    peer.send(b"printf 'BUDGET-'; stty size\r");
    peer.until("BUDGET-21 100");
    let cwd = root.path().join("nested");
    fs::create_dir(&cwd).unwrap();
    peer.send(format!("cd '{}'\r", cwd.to_str().unwrap().replace('\'', "'\\''")).as_bytes());
    peer.until(&format!("cwd~{}", cwd.display()));
    fs::write(
        &path,
        CONFIG
            .replace("Asia/Shanghai", "local")
            .replace("C-g", "C-q"),
    )
    .unwrap();
    peer.send(b"\x07r");
    peer.until("Configuration unchanged");
    fs::write(
        &path,
        CONFIG
            .replace("enabled = true", "enabled = false")
            .replace("separator = true", "separator = false")
            .replace("C-g", "C-q"),
    )
    .unwrap();
    peer.send(b"\x07r");
    peer.until("Configuration reloaded");
    budget(&fixture, &mut peer, false, 23);
    peer.send(b"printf 'FULL-'; stty size\r");
    peer.until("FULL-23 100");
    peer.send(b"\x11d");
    peer.until("RESTORED:0");
    assert_eq!(peer.engine.modes().keyboard, 0);
}

#[test]
fn tiny() {
    let fixture = Fixture::new();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("bagua.toml");
    let mut peer = configured(&fixture, &path, Size::new(160, 3).unwrap());
    peer.until("Header reservation unavailable");
    budget(&fixture, &mut peer, false, 1);
    peer.send(b"printf 'TINY-%s' \"$(stty size)\"\r");
    peer.until("TINY-1 160");
    peer.detach();
}

#[test]
fn traffic() {
    let fixture = Fixture::new();
    let script = r#"stty -echo; for n in $(seq 1 60); do printf '\rCOUNTER%04d' "$n"; sleep 0.02; done; printf '\r\nCOMPLETE\r\n'; exec /bin/sleep 60"#;
    let mut peer = Peer::new(
        &fixture,
        &[
            "open",
            "--executable",
            "/bin/sh",
            "--cwd",
            "/",
            "--arg=-c",
            "--arg",
            script,
            "--env",
            "PATH=/usr/bin:/bin",
            "--env",
            "TERM=xterm-256color",
        ],
        Size::new(80, 24).unwrap(),
    );
    peer.until("COMPLETE");
    assert!(peer.text().contains("COUNTER0060"));
    assert!(
        peer.output.len() < 100_000,
        "unchanged rows were repainted: {} bytes",
        peer.output.len()
    );
    peer.detach();
}
