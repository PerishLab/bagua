use clap::{Args as Group, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "bagua", version = plumb::version!("BAGUA"))]
#[command(
    about = "A persistent terminal workspace. Current source: models only; no terminal hosting."
)]
pub(crate) struct Args {
    #[arg(
        long,
        global = true,
        help = "Emit structured capability or error output"
    )]
    pub json: bool,
    #[arg(
        long,
        global = true,
        default_value_t = 1,
        help = "Nonzero request correlation number"
    )]
    pub request: u64,
    #[command(subcommand)]
    pub action: Action,
}

#[derive(Group)]
pub(crate) struct Dimensions {
    #[arg(long, default_value_t = 80)]
    pub columns: u16,
    #[arg(long, default_value_t = 24)]
    pub rows: u16,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum Axis {
    Columns,
    Rows,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum Kind {
    Session,
    Window,
    Pane,
}

#[derive(Subcommand)]
pub(crate) enum Action {
    #[command(about = "Inspect the exact implemented capability scope")]
    Capabilities,
    #[command(about = "Subscribe to one exact target (not implemented)")]
    Watch {
        #[arg(value_enum)]
        kind: Kind,
        id: u64,
    },
    #[command(about = "Cancel one subscription (not implemented)")]
    Unwatch {
        #[arg(value_enum)]
        kind: Kind,
        id: u64,
    },
    #[command(about = "Create a session (not implemented in this source baseline)")]
    New {
        #[command(flatten)]
        size: Dimensions,
    },
    #[command(about = "List sessions (not implemented)")]
    List,
    #[command(about = "Join as an observer (not implemented)")]
    Attach {
        session: u64,
        #[command(flatten)]
        size: Dimensions,
    },
    #[command(about = "Acquire control, optionally with explicit fixed geometry (not implemented)")]
    Acquire {
        session: u64,
        #[arg(long, requires_all = ["columns", "rows"])]
        fixed: bool,
        #[arg(long, requires = "fixed")]
        columns: Option<u16>,
        #[arg(long, requires = "fixed")]
        rows: Option<u16>,
    },
    #[command(about = "Transfer control to a connected client (not implemented)")]
    Transfer { session: u64, client: u64 },
    #[command(about = "Leave a session (not implemented)")]
    Detach { session: u64 },
    #[command(about = "Add a window (not implemented)")]
    Window { session: u64 },
    #[command(about = "Split a pane (not implemented)")]
    Split {
        pane: u64,
        #[arg(long, value_enum)]
        axis: Axis,
        #[arg(long, default_value_t = 500)]
        ratio: u16,
    },
    #[command(about = "Select a pane and its window (not implemented)")]
    Focus { pane: u64 },
    #[command(about = "Close one exact object (not implemented)")]
    Close {
        #[arg(value_enum)]
        kind: Kind,
        id: u64,
    },
    #[command(about = "Report this client's viewport (not implemented)")]
    Resize {
        session: u64,
        #[command(flatten)]
        size: Dimensions,
    },
    #[command(about = "Set automatic/fixed geometry policy (not implemented)")]
    Size {
        session: u64,
        #[arg(long)]
        fixed: bool,
        #[command(flatten)]
        size: Dimensions,
    },
    #[command(about = "Submit text or paste to a pane (not implemented)")]
    Send {
        pane: u64,
        text: String,
        #[arg(long)]
        paste: bool,
    },
}
