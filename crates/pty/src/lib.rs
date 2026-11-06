mod error;
mod launch;
mod lifecycle;
mod types;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod linux;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod unsupported;

pub use error::{Failure, Stage};
pub use launch::Launch;
pub use lifecycle::{Event, Lifecycle, State, Transition};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use linux::Process;
pub use types::{CHUNK, Exit, GRACE, Interest, Ready, Size, Status, Transfer};
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use unsupported::Process;

pub const SUPPORTED: bool = cfg!(all(target_os = "linux", target_arch = "x86_64"));
