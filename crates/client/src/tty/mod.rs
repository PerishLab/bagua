#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
mod console;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod linux;
mod output;
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
mod windows;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use linux::Terminal;
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub use windows::Terminal;

pub(super) const ENTER: &[u8] =
    b"\x1b[?1049h\x1b[>1u\x1b[?25l\x1b[?7l\x1b[?2004h\x1b[?1003l\x1b[?1002h\x1b[?1006h";
pub(super) const LEAVE: &[u8] =
    b"\x1b[?2026l\x1b[0m\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?2004l\x1b[?7h\x1b[?25h\x1b[<u\x1b[?1049l";
