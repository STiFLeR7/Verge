//! Linux (X11) implementations of `verge_core::ports` capability contracts.

mod credential_store;
#[cfg(any(target_os = "linux", test))]
mod interaction;

#[cfg(target_os = "linux")]
mod overlay;

pub use credential_store::LinuxCredentialStore;

#[cfg(target_os = "linux")]
pub use overlay::X11OverlaySurface;
