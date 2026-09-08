//! Linux (X11) implementations of `verge_core::ports` capability contracts.

mod credential_store;

#[cfg(unix)]
mod overlay;

pub use credential_store::LinuxCredentialStore;

#[cfg(unix)]
pub use overlay::X11OverlaySurface;
