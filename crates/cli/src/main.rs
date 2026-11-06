mod args;
mod command;

use args::Args;
use clap::Parser;
use protocol::{Failure, Request};
use std::process::ExitCode;

fn main() -> ExitCode {
    if let Err(error) = plumb::identity!("BAGUA") {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }
    let args = Args::parse();
    match execute(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            report(error, args.json, args.request);
            ExitCode::from(2)
        }
    }
}

fn execute(args: &Args) -> Result<(), Failure> {
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
        println!("Terminal hosting: absent");
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

fn report(error: Failure, json: bool, request: u64) {
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
