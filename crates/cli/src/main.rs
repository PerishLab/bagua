mod args;
mod command;
mod config;
mod error;
mod remote;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod serve;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod session;
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
    let result = config::load(&args, &|name| std::env::var_os(name)).and_then(|config| {
        config.apply(&mut args);
        execute(&args, config)
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            report(error, args.json, args.request);
            ExitCode::from(2)
        }
    }
}

fn execute(args: &Args, config: config::Loaded) -> Result<(), Error> {
    if matches!(args.action, args::Action::Config) {
        println!("{}", config.report(args));
        return Ok(());
    }
    if args.socket.is_some() || matches!(args.action, args::Action::Serve | args::Action::Control) {
        return remote::execute(args, config);
    }
    let command = command::parse(&args.action)?;
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
