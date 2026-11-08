mod directory;

use crate::{
    args::{Action, Args},
    config::Loaded,
    error::Error,
    session::Session,
};
use bagua::ipc::Connection;
use radicula_process::Environment;
use std::{
    io,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub(crate) struct Invocation {
    pub image: protocol::wire::Image,
    pub environment: Environment,
}

pub(crate) fn connect(path: &Path, args: &Args, config: &Loaded) -> Result<Session, Error> {
    let invocation = config
        .invocation
        .as_ref()
        .ok_or(protocol::Failure::Unavailable)?;
    let create = matches!(
        args.selected(),
        Action::Enter {
            existing: false,
            observe: false,
            ..
        }
    );
    let path = directory::location(path, config.runtime().is_some(), create)?;
    match Connection::connect(&path) {
        Ok(connection) => return adopt(connection, args, invocation),
        Err(error) if error.kind() == io::ErrorKind::NotFound && create => {}
        Err(error) => return Err(error.into()),
    }
    match std::fs::symlink_metadata(&path) {
        Ok(_) => return adopt(Connection::connect(&path)?, args, invocation),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    start(&path, config, invocation)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match Connection::connect(&path) {
            Ok(connection) => return adopt(connection, args, invocation),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        if Instant::now() >= deadline {
            return Err(Error::endpoint(
                "host startup outcome is unknown; inspect the endpoint before another attempt",
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn adopt(connection: Connection, args: &Args, invocation: &Invocation) -> Result<Session, Error> {
    connection.bounded(Duration::from_secs(2))?;
    let session = Session::adopt(connection, args)?;
    if session.identity != Some(invocation.image) {
        return Err(Error::endpoint(
            "host uses a different executable file; endpoint was not replaced",
        ));
    }
    session.bounded(Duration::from_secs(10))?;
    Ok(session)
}

fn start(path: &Path, config: &Loaded, invocation: &Invocation) -> Result<(), Error> {
    let mut command = Command::new("/proc/self/exe");
    invocation.environment.apply(&mut command);
    if let Some(source) = config.source() {
        command.arg("--config").arg(source);
    } else {
        command.arg("--no-config");
    }
    command
        .arg("--socket")
        .arg(path)
        .args(["serve", "--orphan"]);
    command
        .current_dir("/")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = command.spawn()?;
    drop(
        std::thread::Builder::new()
            .name("bagua-host".into())
            .spawn(move || child.wait())
            .map_err(|_| Error::endpoint("host was spawned but wait observation failed; startup outcome is unknown; inspect the endpoint before another attempt"))?,
    );
    Ok(())
}

pub(crate) fn directory(path: &Path, managed: bool) -> Result<std::path::PathBuf, Error> {
    Ok(directory::location(path, managed, true)?)
}
