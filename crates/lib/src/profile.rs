use protocol::{Capability, Component, Profile, State};

pub const EXCLUDED: [&str; 8] = [
    "tmux command and target syntax compatibility",
    "scriptable tmux configuration",
    "format expressions and shell status evaluation",
    "tmux plugin and hook compatibility",
    "non-UTF-8 and ancient terminal compatibility",
    "broad historical Unix portability",
    "implicit multi-scope option inheritance",
    "server-side client-specific rendering",
];

pub fn profile() -> Profile {
    let models = [
        Component::Terminal,
        Component::Pty,
        Component::Workspace,
        Component::Protocol,
        Component::Client,
        Component::Host,
    ];
    let mut components = models
        .into_iter()
        .map(|component| Capability {
            component,
            state: if pty::SUPPORTED {
                State::Available
            } else {
                State::Model
            },
        })
        .collect::<Vec<_>>();
    components.push(Capability {
        component: Component::Cli,
        state: State::Available,
    });
    Profile {
        runtime: pty::SUPPORTED,
        components,
    }
}
