use std::{fmt, io};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Open,
    Launch,
    Read,
    Write,
    Poll,
    Resize,
    Wait,
    Close,
}

#[derive(Debug)]
pub enum Failure {
    Unsupported,
    Input(&'static str),
    Closed,
    Io { stage: Stage, cause: io::Error },
}

impl Failure {
    pub(crate) fn io(stage: Stage, cause: impl Into<io::Error>) -> Self {
        Self::Io {
            stage,
            cause: cause.into(),
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::Unsupported => "unsupported",
            Self::Input(_) => "input",
            Self::Closed => "closed",
            Self::Io { .. } => "io",
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => {
                formatter.write_str("native PTY backend is unqualified on this platform")
            }
            Self::Input(reason) => formatter.write_str(reason),
            Self::Closed => formatter.write_str("PTY master is closed"),
            Self::Io { stage, cause } => write!(formatter, "{stage:?}: {cause}"),
        }
    }
}

impl std::error::Error for Failure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { cause, .. } => Some(cause),
            _ => None,
        }
    }
}
