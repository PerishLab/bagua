use clap::{Args as Group, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "bagua", version = plumb::version!("BAGUA"))]
#[command(
    about = "A persistent terminal workspace with Linux machine control and interactive clients."
)]
pub(crate) struct Args {
    #[arg(
        long,
        global = true,
        help = "Emit structured capability or error output"
    )]
    pub json: bool,
    #[arg(long, global = true, help = "Explicit absolute host socket path")]
    pub socket: Option<std::path::PathBuf>,
    #[arg(
        long,
        global = true,
        help = "Explicitly join and acquire this session for this command"
    )]
    pub control: Option<u64>,
    #[arg(
        long,
        id = "observer",
        global = true,
        conflicts_with = "control",
        help = "Join this session as an observer for this command"
    )]
    pub session: Option<u64>,
    #[arg(
        long,
        global = true,
        default_value_t = 1,
        help = "First nonzero request number; explicit session setup consumes additional numbers"
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
    #[command(about = "Open a session, explicitly launch a program and enter interactive control")]
    Open {
        #[command(flatten)]
        launch: Launch,
    },
    #[command(about = "Connect an interactive observer; --take explicitly acquires control")]
    Connect {
        session: u64,
        #[arg(long)]
        take: bool,
        #[arg(long, requires = "take")]
        fixed: bool,
        #[command(flatten)]
        launch: Launch,
    },
    #[command(about = "Run the persistent host in the foreground")]
    Serve,
    #[command(about = "Exchange JSON requests and messages over one persistent connection")]
    Control,
    #[command(about = "Gracefully stop this host and reclaim its owned processes")]
    Shutdown,
    #[command(about = "Read a complete terminal snapshot from a joined session")]
    Inspect { pane: u64 },
    #[command(about = "Start an explicit program in a pending pane")]
    Start {
        pane: u64,
        #[arg(long)]
        executable: String,
        #[arg(long)]
        cwd: String,
        #[arg(long = "arg", allow_hyphen_values = true)]
        args: Vec<String>,
        #[arg(long = "env")]
        environment: Vec<String>,
    },
    #[command(about = "Submit one explicit terminal key")]
    Key {
        pane: u64,
        #[arg(value_enum)]
        key: Key,
    },
    #[command(about = "Inspect the exact implemented capability scope")]
    Capabilities,
    #[command(about = "Subscribe to one exact target")]
    Watch {
        #[arg(value_enum)]
        kind: Kind,
        id: u64,
    },
    #[command(about = "Cancel one subscription")]
    Unwatch {
        #[arg(value_enum)]
        kind: Kind,
        id: u64,
    },
    #[command(about = "Create a session")]
    New {
        #[command(flatten)]
        size: Dimensions,
    },
    #[command(about = "List sessions")]
    List,
    #[command(about = "Join as an observer")]
    Attach {
        session: u64,
        #[command(flatten)]
        size: Dimensions,
    },
    #[command(about = "Acquire control, optionally with explicit fixed geometry")]
    Acquire {
        session: u64,
        #[arg(long, requires_all = ["columns", "rows"])]
        fixed: bool,
        #[arg(long, requires = "fixed")]
        columns: Option<u16>,
        #[arg(long, requires = "fixed")]
        rows: Option<u16>,
    },
    #[command(about = "Transfer control to a connected client")]
    Transfer { session: u64, client: u64 },
    #[command(about = "Leave a session")]
    Detach { session: u64 },
    #[command(about = "Add a window")]
    Window { session: u64 },
    #[command(about = "Split a pane")]
    Split {
        pane: u64,
        #[arg(long, value_enum)]
        axis: Axis,
        #[arg(long, default_value_t = 500)]
        ratio: u16,
    },
    #[command(about = "Select a pane and its window")]
    Focus { pane: u64 },
    #[command(about = "Close one exact object")]
    Close {
        #[arg(value_enum)]
        kind: Kind,
        id: u64,
    },
    #[command(about = "Report this client's viewport")]
    Resize {
        session: u64,
        #[command(flatten)]
        size: Dimensions,
    },
    #[command(about = "Set automatic/fixed geometry policy")]
    Size {
        session: u64,
        #[arg(long)]
        fixed: bool,
        #[command(flatten)]
        size: Dimensions,
    },
    #[command(about = "Submit text or paste to a pane")]
    Send {
        pane: u64,
        text: String,
        #[arg(long)]
        paste: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum Key {
    Enter,
    Tab,
    Escape,
    Backspace,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
}

#[derive(Group)]
pub(crate) struct Launch {
    #[arg(long, requires = "cwd")]
    pub executable: Option<String>,
    #[arg(long, requires = "executable")]
    pub cwd: Option<String>,
    #[arg(long = "arg", allow_hyphen_values = true, requires = "executable")]
    pub args: Vec<String>,
    #[arg(long = "env", requires = "executable")]
    pub environment: Vec<String>,
}
