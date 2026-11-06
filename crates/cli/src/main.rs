use clap::{Parser, Subcommand};
use protocol::{Command, Failure, Request};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "bagua", version = plumb::version!("BAGUA"))]
#[command(
    about = "A persistent terminal workspace. Current source: models only; no terminal hosting."
)]
struct Args {
    #[arg(
        long,
        global = true,
        help = "Emit structured capability or error output"
    )]
    json: bool,
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    #[command(about = "Inspect the exact implemented capability scope")]
    Capabilities,
    #[command(about = "Create a session (not implemented in this source baseline)")]
    New,
    #[command(about = "List sessions (not implemented in this source baseline)")]
    List,
    #[command(about = "Attach to a session (not implemented in this source baseline)")]
    Attach { session: u64 },
}

fn main() -> ExitCode {
    if let Err(error) = plumb::identity!("BAGUA") {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }
    let args = Args::parse();
    match execute(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            report(error, args.json);
            ExitCode::from(2)
        }
    }
}

fn execute(args: &Args) -> Result<(), Failure> {
    let command = match &args.action {
        Action::Capabilities => Command::Capabilities,
        Action::New => Command::Create,
        Action::List => Command::Sessions,
        Action::Attach { session } => Command::attach(*session)?,
    };
    let profile = bagua::dispatch(Request::new(command))?;
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

fn report(error: Failure, json: bool) {
    if json {
        eprintln!(
            "{}",
            serde_json::json!({
                "error": error.code(),
                "message": error.message(),
            })
        );
    } else {
        eprintln!("{}: {}", error.code(), error.message());
    }
}
