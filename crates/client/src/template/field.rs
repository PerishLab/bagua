use protocol::Failure;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Session,
    Window,
    Name,
    Title,
    Selected,
    Pane,
    Identity,
    Command,
    Directory,
    Leaf,
    Active,
    State,
    Role,
    Mode,
    Columns,
    Copy,
    Recovery,
    Observer,
    Prefix,
    Waiting,
    Notice,
    Clock,
    Date,
}

impl Field {
    pub(super) fn parse(text: &str) -> Result<Self, Failure> {
        Ok(match text {
            "session.name" => Self::Session,
            "window.index" => Self::Window,
            "window.name" => Self::Name,
            "window.title" => Self::Title,
            "window.active" => Self::Selected,
            "pane.index" => Self::Pane,
            "pane.id" => Self::Identity,
            "pane.command" => Self::Command,
            "pane.cwd" => Self::Directory,
            "pane.cwd_name" => Self::Leaf,
            "pane.active" => Self::Active,
            "pane.state" => Self::State,
            "client.role" => Self::Role,
            "client.mode" => Self::Mode,
            "client.columns" => Self::Columns,
            "client.copy" => Self::Copy,
            "client.recovering" => Self::Recovery,
            "client.observe" => Self::Observer,
            "prefix.key" => Self::Prefix,
            "prefix.active" => Self::Waiting,
            "notice.text" => Self::Notice,
            "clock.time" => Self::Clock,
            "clock.date" => Self::Date,
            _ => return Err(Failure::Payload),
        })
    }

    pub(super) fn numeric(self) -> bool {
        matches!(
            self,
            Self::Window | Self::Pane | Self::Identity | Self::Columns
        )
    }

    pub(super) fn metadata(self) -> bool {
        matches!(
            self,
            Self::Command | Self::Directory | Self::Leaf | Self::Title
        )
    }
}
