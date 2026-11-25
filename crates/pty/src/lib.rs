mod error;
mod launch;
mod lifecycle;
mod types;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod linux;
#[cfg(not(any(
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "windows", target_arch = "x86_64")
)))]
mod unsupported;
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
mod windows;

pub use error::{Failure, Stage};
pub use launch::Launch;
pub use lifecycle::{Event, Lifecycle, State, Transition};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use linux::Process;
pub use types::{CHUNK, Context, Exit, GRACE, Interest, Ready, Size, Status, Transfer};
#[cfg(not(any(
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "windows", target_arch = "x86_64")
)))]
pub use unsupported::Process;
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub use windows::Process;

pub const SUPPORTED: bool = cfg!(any(
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "windows", target_arch = "x86_64")
));
