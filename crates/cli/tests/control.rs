#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

use bagua::ipc::Service;
use protocol::{Command, Request, wire::Program};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    process::{Command as Process, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use workspace::{Geometry, Id, Operation};

struct Stop(Arc<AtomicBool>);
impl Drop for Stop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

fn directory() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
    directory
}

#[test]
fn control() {
    let directory = directory();
    let path = directory.path().join("host.sock");
    let mut service = Service::bind(&path).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let session = Id::new(1).unwrap();
    let commands = [
        Command::create(80, 24).unwrap(),
        Command::Workspace(Operation::Join {
            session,
            viewport: Geometry::new(80, 24).unwrap(),
        }),
        Command::Workspace(Operation::Acquire {
            session,
            fixed: None,
        }),
        Command::Start {
            pane: Id::new(3).unwrap(),
            program: Program {
                executable: "/bin/true".into(),
                args: Vec::new(),
                cwd: "/".into(),
                environment: BTreeMap::new(),
            },
        },
    ];
    std::thread::scope(|scope| {
        let task = scope.spawn(|| service.run(&stop).unwrap());
        let guard = Stop(Arc::clone(&stop));
        let mut child = Process::new(env!("CARGO_BIN_EXE_bagua"))
            .arg("--socket")
            .arg(&path)
            .args(["control", "--json"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        for (index, command) in commands.into_iter().enumerate() {
            let request = Request::identified(index as u64 + 1, command).unwrap();
            writeln!(input, "{}", serde_json::to_string(&request).unwrap()).unwrap();
        }
        drop(input);
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let messages: Vec<serde_json::Value> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert!(
            messages
                .iter()
                .any(|m| m["Finished"]["request"] == 4
                    && m["Finished"]["result"].get("Ok").is_some())
        );
        drop(guard);
        task.join().unwrap();
    });
    drop(service);
    assert!(!path.exists());
}
