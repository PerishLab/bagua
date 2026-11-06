mod queue;
mod runner;
mod setup;

use crate::{args::Args, error::Error};
use std::path::Path;

pub(crate) fn run(path: &Path, args: &Args) -> Result<(), Error> {
    if args.json || args.control.is_some() || args.session.is_some() {
        return Err(protocol::Failure::Payload.into());
    }
    let (_signals, stop) = crate::serve::listen()?;
    let mut terminal = client::Terminal::enter()?;
    let result = setup::prepare(path, args, &terminal)
        .and_then(|mut runner| runner.run(&mut terminal, &stop));
    let restored = terminal.finish();
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

pub(super) fn extent(columns: u16, rows: u16) -> Option<terminal::Extent> {
    if columns < 2 || rows < 2 {
        return None;
    }
    terminal::Extent::new(columns.min(512), (rows - 1).min(255))
}
