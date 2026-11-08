mod app;
pub use select::Copy;
mod draw;
mod keyboard;
mod keys;
mod mouse;
pub use app::preferences::{Clipboard, Decor, Header, Mouse, Status};
pub use keyboard::Keyboard;
mod select;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod tty;

pub use app::App;
pub use draw::Paint;
pub use keys::{Gesture, Keys, translate};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use tty::Terminal;

use protocol::{Failure, Frame, Packet, Replica};
use terminal::{Extent, Revision};

#[derive(Debug)]
pub struct View {
    extent: Extent,
    state: Replica<terminal::Snapshot>,
}

impl View {
    pub fn subscribe(&self, request: u64) -> Result<protocol::Request, Failure> {
        protocol::Request::identified(
            request,
            protocol::Command::Subscribe {
                target: self.state.target(),
                after: self.revision(),
            },
        )
    }

    pub fn unsubscribe(&self, request: u64) -> Result<protocol::Request, Failure> {
        protocol::Request::identified(request, protocol::Command::Unsubscribe(self.state.target()))
    }
    pub fn input(
        &self,
        request: u64,
        input: protocol::Input,
    ) -> Result<protocol::Request, Failure> {
        let workspace::Target::Pane(pane) = self.state.target() else {
            return Err(Failure::Target);
        };
        protocol::Request::identified(request, protocol::Command::Send { pane, input })
    }

    pub fn focus(&self, request: u64) -> Result<protocol::Request, Failure> {
        let workspace::Target::Pane(pane) = self.state.target() else {
            return Err(Failure::Target);
        };
        protocol::Request::identified(
            request,
            protocol::Command::Workspace(workspace::Operation::Focus(pane)),
        )
    }
    pub fn new(target: workspace::Target, extent: Extent) -> Self {
        Self {
            extent,
            state: Replica::new(target),
        }
    }

    pub fn resize(&mut self, extent: Extent) {
        self.extent = extent;
    }

    pub fn crop(&self, content: Extent, x: u16, y: u16) -> Option<Extent> {
        Some(content.remaining(x, y)?.fit(self.extent))
    }

    pub fn accept(&mut self, frame: Frame<terminal::Snapshot>) -> Result<(), Failure> {
        if !matches!(self.state.target(), workspace::Target::Pane(_)) {
            return Err(Failure::Target);
        }
        self.state.apply(Packet::<_, ()>::Snapshot(frame), |_, ()| {
            Err(Failure::Payload)
        })
    }

    pub fn state(&self) -> Option<&terminal::Snapshot> {
        self.state.state()
    }

    pub fn revision(&self) -> Option<Revision> {
        self.state.revision()
    }
}
