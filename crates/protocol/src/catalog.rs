#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Component {
    Terminal,
    Pty,
    Workspace,
    Protocol,
    Client,
    Host,
    Cli,
}

impl Component {
    pub fn name(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Pty => "pty",
            Self::Workspace => "workspace",
            Self::Protocol => "protocol",
            Self::Client => "client",
            Self::Host => "host",
            Self::Cli => "cli",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Model,
    Available,
    Absent,
    Excluded,
    Unverified,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            Self::Model => "model",
            Self::Available => "available",
            Self::Absent => "absent",
            Self::Excluded => "excluded",
            Self::Unverified => "unverified",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capability {
    pub component: Component,
    pub state: State,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    pub runtime: bool,
    pub components: Vec<Capability>,
}
