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
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub(crate) use directory::image;

#[derive(Clone)]
pub(crate) struct Invocation {
    pub executable: PathBuf,
    pub image: protocol::wire::Image,
    pub environment: Environment,
}

#[locus::trace(with = crate::observation::view())]
pub(crate) fn connect(path: &Path, args: &Args, config: &Loaded) -> Result<Session, Error> {
    let invocation = config
        .invocation
        .as_ref()
        .ok_or(protocol::Failure::Unavailable)?;
    let create = !args.observe
        && matches!(
            args.selected(),
            Action::Enter {
                existing: false,
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

#[locus::trace(with = crate::observation::view(), frame = context)]
fn start(path: &Path, config: &Loaded, invocation: &Invocation) -> Result<(), Error> {
    let mut command = Command::new(&invocation.executable);
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
    configure(&mut command, path);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    crate::observation::parent(&mut command, context.as_ref());
    let mut child = command.spawn()?;
    drop(
        std::thread::Builder::new()
            .name("bagua-host".into())
            .spawn(move || child.wait())
            .map_err(|_| Error::endpoint("host was spawned but wait observation failed; startup outcome is unknown; inspect the endpoint before another attempt"))?,
    );
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn configure(command: &mut Command, path: &Path) {
    let _ = path.as_os_str();
    command.current_dir("/");
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
fn configure(command: &mut Command, path: &Path) {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::{
        CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW, DETACHED_PROCESS,
    };
    let cwd = path.parent().unwrap_or_else(|| Path::new("."));
    command
        .current_dir(cwd)
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
}

pub(crate) fn location(path: &Path, managed: bool, create: bool) -> Result<PathBuf, Error> {
    Ok(directory::location(path, managed, create)?)
}
