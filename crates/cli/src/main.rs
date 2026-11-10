mod args;
mod command;
mod config;
mod error;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "startup/observation/mod.rs"]
mod observation;
mod remote;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod serve;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod session;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod startup;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod tui;

use args::Args;
use clap::Parser;
use error::Error;
use protocol::Request;
use std::process::ExitCode;

fn main() -> ExitCode {
    if let Err(error) = plumb::identity!("BAGUA") {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }
    let mut args = Args::parse();
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let mut observation = None;
    let result = config::load(&args, &|name| std::env::var_os(name)).and_then(|mut config| {
        config.apply(&mut args);
        defaults(&mut args, &config)?;
        config::validate(&args)?;
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        initialize(&args, &mut config)?;
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            let active = observe(&args, &config);
            config.notes = active.notes.clone();
            observation = Some(active);
        }
        execute(&args, config)
    });
    let code = match result {
        Ok(()) => 0,
        Err(error) => {
            report(error, args.json, args.request);
            2
        }
    };
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    if let Some(observation) = observation {
        observation.finish(code, args.json);
    }
    ExitCode::from(code as u8)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn observe(args: &Args, config: &config::Loaded) -> observation::Observer {
    let get = |name| std::env::var_os(name).filter(|value| !value.is_empty());
    let state = get("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| get("HOME").map(|home| std::path::PathBuf::from(home).join(".local/state")));
    let command = match args.selected() {
        args::Action::Serve { .. } => "host",
        args::Action::Control => "control",
        args::Action::Config => "config",
        _ if args.socket.is_some() => "client",
        _ => "local",
    };
    observation::Observer::start(
        config,
        command,
        observation::Inputs {
            state,
            endpoint: std::env::var_os("LOCUS_API"),
            parent: std::env::var_os("LOCUS_PARENT"),
        },
    )
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn initialize(args: &Args, config: &mut config::Loaded) -> Result<(), Error> {
    use std::os::unix::fs::MetadataExt;
    if !matches!(
        args.selected(),
        args::Action::Enter { .. } | args::Action::Serve { .. }
    ) {
        return Ok(());
    }
    let running = std::fs::metadata("/proc/self/exe")?;
    let image = protocol::wire::Image {
        device: running.dev(),
        inode: running.ino(),
    };
    let environment = radicula_process::Environment::new(
        [
            "HOME",
            "PATH",
            "XDG_CONFIG_HOME",
            "XDG_RUNTIME_DIR",
            "XDG_STATE_HOME",
            "LOCUS_API",
            "LANG",
            "LC_ALL",
            "LC_CTYPE",
            "TZ",
        ]
        .into_iter()
        .filter_map(|key| std::env::var_os(key).map(|value| (key.into(), value))),
    );
    config.invocation = Some(startup::Invocation { image, environment });
    Ok(())
}

fn defaults(args: &mut Args, config: &config::Loaded) -> Result<(), Error> {
    if args.observe
        || !matches!(args.selected(), args::Action::Enter { launch, .. } if launch.executable.is_none())
    {
        return Ok(());
    }
    let mut environment = std::collections::BTreeMap::new();
    for (key, value) in std::env::vars_os() {
        let key = key
            .into_string()
            .map_err(|_| Error::config("interactive default environment key must be UTF-8"))?;
        let value = value
            .into_string()
            .map_err(|_| Error::config("interactive default environment value must be UTF-8"))?;
        environment.insert(key, value);
    }
    let executable = environment
        .get("SHELL")
        .cloned()
        .unwrap_or_else(|| "/bin/sh".into());
    environment.insert("TERM".into(), "xterm-256color".into());
    let cwd = std::env::current_dir()?
        .into_os_string()
        .into_string()
        .map_err(|_| Error::config("interactive default cwd must be UTF-8"))?;
    config.defaults(
        args,
        protocol::wire::Program {
            executable,
            args: vec!["-i".into()],
            cwd,
            environment,
        },
    )
}

#[cfg_attr(all(target_os = "linux", target_arch = "x86_64"), locus::trace(with = observation::view()))]
fn execute(args: &Args, config: config::Loaded) -> Result<(), Error> {
    if matches!(args.selected(), args::Action::Config) {
        println!("{}", config.report(args));
        return Ok(());
    }
    if args.socket.is_some()
        || matches!(
            args.selected(),
            args::Action::Serve { .. } | args::Action::Control
        )
    {
        return remote::execute(args, config);
    }
    let command = command::parse(args.selected())?;
    let profile = bagua::dispatch(Request::identified(args.request, command)?)?;
    if args.json {
        let components = profile
            .components
            .iter()
            .map(|capability| {
                serde_json::json!({
                    "name": capability.component.name(),
                    "state": capability.state.name(),
                })
            })
            .collect::<Vec<_>>();
        println!(
            "{}",
            serde_json::json!({
                "version": protocol::VERSION,
                "request": args.request,
                "runtime": profile.runtime,
                "components": components,
                "excluded": bagua::EXCLUDED,
            })
        );
    } else {
        println!("Terminal hosting available: {}", profile.runtime);
        for capability in profile.components {
            println!(
                "{}: {}",
                capability.component.name(),
                capability.state.name()
            );
        }
    }
    Ok(())
}

fn report(error: Error, json: bool, request: u64) {
    let request = error.correlation(request);
    if json {
        eprintln!(
            "{}",
            serde_json::json!({
                "error": error.code(),
                "request": request,
                "message": error.message(),
            })
        );
    } else {
        eprintln!("{}: {}", error.code(), error.message());
    }
}
