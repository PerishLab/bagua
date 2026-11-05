mod host;
mod profile;

pub use host::Host;
pub use profile::{EXCLUDED, profile};
use protocol::{Command, Failure, Profile, Request};

pub fn dispatch(request: Request) -> Result<Profile, Failure> {
    match request.check()? {
        Command::Capabilities => Ok(profile()),
        Command::Create | Command::Sessions | Command::Attach { .. } => Err(Failure::Unavailable),
    }
}
