#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod channel;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use channel::Channel;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod connection;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod link;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod service;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod socket;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use connection::Connection;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use service::Service;

pub fn error(error: protocol::Failure) -> std::io::Error {
    std::io::Error::other(format!("{}: {}", error.code(), error.message()))
}
