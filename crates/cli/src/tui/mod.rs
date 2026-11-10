mod queue;
mod runner;
mod setup;

use crate::{args::Args, error::Error};
use std::path::Path;

#[locus::trace(with = crate::observation::view())]
pub(crate) fn run(path: &Path, args: &Args, config: crate::config::Loaded) -> Result<(), Error> {
    if args.request == 0 {
        return Err(protocol::Failure::Identity.into());
    }
    if args.json || args.control.is_some() || args.session.is_some() {
        return Err(protocol::Failure::Payload.into());
    }
    let (_signals, stop) = crate::serve::listen()?;
    let notes = config.notes.clone();
    notes.safe(false);
    let mut terminal = client::Terminal::enter()?;
    let connection = if matches!(args.selected(), crate::args::Action::Enter { .. }) {
        crate::startup::connect(path, args, &config)
    } else {
        crate::session::Session::connect(path, args)
    };
    let result = connection
        .and_then(|rpc| {
            setup::prepare(
                rpc,
                args,
                &mut setup::Display {
                    terminal: &mut terminal,
                    stop: &stop,
                    separator: config.decor().status.separator,
                },
                &config,
            )
        })
        .and_then(|runner| match runner {
            Some(runner) => runner.configure(config).run(&mut terminal, &stop),
            None => Ok(()),
        });
    let restored = terminal.finish();
    notes.safe(restored.is_ok());
    crate::observation::event::terminal(restored.is_ok());
    match (result, restored) {
        (Err(error), Err(cleanup)) => {
            eprintln!("bagua terminal restoration failed: {cleanup}");
            Err(error)
        }
        (Err(error), _) => Err(error),
        (_, Err(error)) => Err(error.into()),
        _ => Ok(()),
    }
}

pub(super) fn extent(columns: u16, rows: u16, separator: bool) -> Option<terminal::Extent> {
    let reserved = 1 + u16::from(separator);
    if columns < 2 || rows <= reserved {
        return None;
    }
    terminal::Extent::new(columns.min(512), (rows - reserved).min(256 - reserved))
}
