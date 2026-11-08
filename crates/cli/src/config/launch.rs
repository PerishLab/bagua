use crate::{
    args::{Action, Args},
    error::Error,
};
use protocol::{Failure, wire::Program};
use std::path::Path;

pub(super) fn apply(args: &mut Args, program: &Option<Program>) {
    let launch = match args.action.get_or_insert_with(Action::ordinary) {
        Action::Open { launch } | Action::Connect { launch, .. } | Action::Enter { launch, .. } => {
            launch
        }
        _ => return,
    };
    if launch.executable.is_some() {
        return;
    }
    if let Some(program) = program {
        launch.executable = Some(program.executable.clone());
        launch.cwd = Some(program.cwd.clone());
        launch.args = program.args.clone();
        launch.environment = program
            .environment
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
    }
}

pub(super) fn check(program: &Program) -> Result<(), Error> {
    if !Path::new(&program.executable).is_absolute() || !Path::new(&program.cwd).is_absolute() {
        return Err(Error::config(
            "program.executable/program.cwd: expected absolute paths",
        ));
    }
    if program.args.len() > 256 || program.environment.len() > 256 {
        return Err(Error::config(
            "program: argument/environment count exceeds 256",
        ));
    }
    let paths = [&program.executable, &program.cwd];
    let values = paths.into_iter().chain(&program.args).chain(
        program
            .environment
            .iter()
            .flat_map(|(key, value)| [key, value]),
    );
    let mut bytes = 0;
    for value in values {
        if value.contains('\0') {
            return Err(Error::config("program: launch data contains NUL"));
        }
        bytes += value.len();
    }
    if bytes > 65_536
        || program
            .environment
            .keys()
            .any(|key| key.is_empty() || key.contains('='))
    {
        return Err(Error::config(
            "program: invalid environment key or launch data exceeds 65536 bytes",
        ));
    }
    let request = protocol::Request::identified(
        u64::MAX,
        protocol::Command::Start {
            pane: workspace::Id::new(u64::MAX).unwrap(),
            program: program.clone(),
        },
    )?;
    protocol::wire::encode(&request, protocol::wire::REQUEST)
        .map_err(|_| Error::config("program: encoded launch exceeds the 128 KiB request budget"))?;
    Ok(())
}

pub(super) fn resolve(launch: &crate::args::Launch) -> Result<Option<Program>, Error> {
    let Some(executable) = &launch.executable else {
        return Ok(None);
    };
    let environment = launch
        .environment
        .iter()
        .map(|entry| {
            let (key, value) = entry.split_once('=').ok_or(Failure::Payload)?;
            Ok((key.into(), value.into()))
        })
        .collect::<Result<_, Failure>>()?;
    let program = Program {
        executable: executable.clone(),
        args: launch.args.clone(),
        cwd: launch.cwd.clone().ok_or(Failure::Payload)?,
        environment,
    };
    check(&program)?;
    Ok(Some(program))
}
